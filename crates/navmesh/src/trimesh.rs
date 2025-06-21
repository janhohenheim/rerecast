use avian3d::{
    parry::shape::{Compound, TypedShape},
    prelude::*,
};
use bevy::prelude::*;

/// A [`Collider`] rasterized into trimesh form.
#[derive(Debug, Clone, PartialEq)]
pub struct TrimeshedCollider {
    /// The vertices composing the collider.
    /// Follows the convention of [`PrimitiveTopology::TriangleList`](bevy::render::mesh::PrimitiveTopology::TriangleList).
    pub vertices: Vec<Vec3>,

    /// The indices composing the collider.
    /// Follows the convention of [`PrimitiveTopology::TriangleList`](bevy::render::mesh::PrimitiveTopology::TriangleList).
    pub indices: Vec<[u32; 3]>,
}

/// A trait for converting a [`Collider`] into a [`TrimeshedCollider`].
pub trait ToTrimesh {
    /// Converts the collider into a [`TrimeshedCollider`].
    ///
    /// # Arguments
    ///
    /// * `subdivisions` - The number of subdivisions to use for the collider. This is used for curved shapes such as circles and spheres.
    ///
    /// # Returns
    ///
    /// A [`TrimeshedCollider`] if the collider is supported, otherwise `None`
    ///
    /// The following shapes are not supported:
    /// - [`Segment`](avian3d::parry::shape::Segment)
    /// - [`Polyline`](avian3d::parry::shape::Polyline)
    /// - [`HalfSpace`](avian3d::parry::shape::HalfSpace)
    /// - Custom shapes
    ///
    /// The following rounded shapes are supported, but only the inner shape without rounding is used:
    /// - [`RoundCuboid`](avian3d::parry::shape::RoundCuboid)
    /// - [`RoundTriangle`](avian3d::parry::shape::RoundTriangle)
    /// - [`RoundConvexPolyhedron`](avian3d::parry::shape::RoundConvexPolyhedron)
    /// - [`RoundCylinder`](avian3d::parry::shape::RoundCylinder)
    /// - [`RoundCone`](avian3d::parry::shape::RoundCone)
    fn to_trimesh(&self, subdivisions: u32) -> Option<TrimeshedCollider>;
}

impl ToTrimesh for Collider {
    fn to_trimesh(&self, subdivisions: u32) -> Option<TrimeshedCollider> {
        shape_to_trimesh(&self.shape().as_typed_shape(), subdivisions)
    }
}

fn shape_to_trimesh(shape: &TypedShape, subdivisions: u32) -> Option<TrimeshedCollider> {
    let (vertices, indices) = match shape {
        // Simple cases
        TypedShape::Cuboid(cuboid) => cuboid.to_trimesh(),
        TypedShape::ConvexPolyhedron(convex_polyhedron) => convex_polyhedron.to_trimesh(),
        TypedShape::HeightField(height_field) => height_field.to_trimesh(),
        // Triangles
        TypedShape::Triangle(triangle) => {
            (vec![triangle.a, triangle.b, triangle.c], vec![[0, 1, 2]])
        }
        TypedShape::TriMesh(tri_mesh) => {
            (tri_mesh.vertices().to_vec(), tri_mesh.indices().to_vec())
        }
        // Need subdivisions
        TypedShape::Ball(ball) => ball.to_trimesh(subdivisions, subdivisions),
        TypedShape::Capsule(capsule) => capsule.to_trimesh(subdivisions, subdivisions),
        TypedShape::Cylinder(cylinder) => cylinder.to_trimesh(subdivisions),
        TypedShape::Cone(cone) => cone.to_trimesh(subdivisions),
        // Compounds need to be unpacked
        TypedShape::Compound(compound) => {
            return Some(compound_trimesh(compound, subdivisions));
        }
        // Rounded shapes ignore the rounding and use the inner shape
        TypedShape::RoundCuboid(round_shape) => round_shape.inner_shape.to_trimesh(),
        TypedShape::RoundTriangle(round_shape) => (
            vec![
                round_shape.inner_shape.a,
                round_shape.inner_shape.b,
                round_shape.inner_shape.c,
            ],
            vec![[0, 1, 2]],
        ),
        TypedShape::RoundConvexPolyhedron(round_shape) => round_shape.inner_shape.to_trimesh(),
        TypedShape::RoundCylinder(round_shape) => round_shape.inner_shape.to_trimesh(subdivisions),
        TypedShape::RoundCone(round_shape) => round_shape.inner_shape.to_trimesh(subdivisions),
        // Not supported
        TypedShape::Segment(_segment) => return None,
        TypedShape::Polyline(_polyline) => return None,
        TypedShape::HalfSpace(_half_space) => return None,
        TypedShape::Custom(_shape) => return None,
    };
    Some(TrimeshedCollider {
        vertices: vertices.into_iter().map(|v| v.into()).collect(),
        indices,
    })
}

fn compound_trimesh(compound: &Compound, subdivisions: u32) -> TrimeshedCollider {
    let mut total_vertices = Vec::new();
    let mut total_indices = Vec::new();

    for (isometry, shape) in compound.shapes() {
        let Some(TrimeshedCollider { vertices, indices }) =
            // No need to track recursive compounds because parry panics on nested compounds anyways lol
            shape_to_trimesh(&shape.as_typed_shape(), subdivisions)
        else {
            continue;
        };

        let translation = Vec3::from(isometry.translation);
        let rotation = Quat::from(isometry.rotation);

        let next_vertex_index = total_vertices.len();
        total_vertices.extend(vertices.iter().map(|v| rotation * *v + translation));
        total_indices.extend(indices.iter().map(|i| {
            [
                i[0] + next_vertex_index as u32,
                i[1] + next_vertex_index as u32,
                i[2] + next_vertex_index as u32,
            ]
        }));
    }
    TrimeshedCollider {
        vertices: total_vertices,
        indices: total_indices,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rasterizes_cuboid() {
        let collider = Collider::cuboid(1.0, 2.0, 3.0);
        let trimesh = collider.to_trimesh(1).unwrap();
        assert_eq!(trimesh.vertices.len(), 8);
        assert_eq!(trimesh.indices.len(), 12);
    }
}
