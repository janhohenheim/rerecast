use avian3d::{
    parry::shape::{Compound, TypedShape},
    prelude::*,
};
use bevy::prelude::*;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct RasterizedCollider {
    pub(crate) vertices: Vec<Vec3>,
    pub(crate) indices: Vec<[u32; 3]>,
}

pub(crate) trait Rasterize {
    fn rasterize(&self, subdivisions: u32) -> Option<RasterizedCollider>;
}

impl Rasterize for Collider {
    fn rasterize(&self, subdivisions: u32) -> Option<RasterizedCollider> {
        shape_to_trimesh(&self.shape().as_typed_shape(), subdivisions)
    }
}

fn shape_to_trimesh(shape: &TypedShape, subdivisions: u32) -> Option<RasterizedCollider> {
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
    Some(RasterizedCollider {
        vertices: vertices.into_iter().map(|v| v.into()).collect(),
        indices,
    })
}

fn compound_trimesh(compound: &Compound, subdivisions: u32) -> RasterizedCollider {
    let mut total_vertices = Vec::new();
    let mut total_indices = Vec::new();

    for (isometry, shape) in compound.shapes() {
        let Some(RasterizedCollider { vertices, indices }) =
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
    RasterizedCollider {
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
        let trimesh = collider.rasterize(1).unwrap();
        assert_eq!(trimesh.vertices.len(), 8);
        assert_eq!(trimesh.indices.len(), 12);
    }
}
