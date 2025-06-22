//! Contains traits and methods for converting [`Collider`]s into trimeshes, expressed as [`TrimeshedCollider`]s.

use std::ops::Mul;

use avian3d::{
    parry::shape::{Compound, TypedShape},
    prelude::*,
};
use bevy::{math::bounding::Aabb3d, prelude::*};

/// A [`Collider`] rasterized into trimesh form.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TrimeshedCollider {
    /// The vertices composing the collider.
    /// Follows the convention of [`PrimitiveTopology::TriangleList`](bevy::render::mesh::PrimitiveTopology::TriangleList).
    pub vertices: Vec<Vec3A>,

    /// The indices composing the collider.
    /// Follows the convention of [`PrimitiveTopology::TriangleList`](bevy::render::mesh::PrimitiveTopology::TriangleList).
    pub indices: Vec<[u32; 3]>,
}

impl TrimeshedCollider {
    /// Extends the trimesh with the vertices and indices of another trimesh.
    /// The indices of `other` will be offset by the number of vertices in `self`.
    pub fn extend(&mut self, other: TrimeshedCollider) {
        if self.vertices.len() > u32::MAX as usize {
            panic!("Cannot extend a trimesh with more than 2^32 vertices");
        }
        let next_vertex_index = self.vertices.len() as u32;
        self.vertices.extend(other.vertices);
        self.indices.extend(other.indices.iter().map(|i| {
            [
                i[0] + next_vertex_index,
                i[1] + next_vertex_index,
                i[2] + next_vertex_index,
            ]
        }));
    }

    /// Applies an isometry to the trimesh.
    pub fn apply_isometry(&mut self, isometry: Isometry3d) {
        self.vertices.iter_mut().for_each(|v| {
            *v = isometry * *v;
        });
    }

    /// Computes the AABB of the trimesh.
    pub fn compute_aabb(&self) -> Aabb3d {
        todo!("Do the manual impl, not Aabb3d::from_point_cloud. We have no isometry!")
    }
}

impl Mul<TrimeshedCollider> for Isometry3d {
    type Output = TrimeshedCollider;

    fn mul(self, mut trimesh: TrimeshedCollider) -> Self::Output {
        trimesh.apply_isometry(self);
        trimesh
    }
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
    compound.shapes().iter().fold(
        TrimeshedCollider::default(),
        |mut compound_trimesh, (isometry, shape)| {
            let Some(trimesh) =
                // No need to track recursive compounds because parry panics on nested compounds anyways lol
                shape_to_trimesh(&shape.as_typed_shape(), subdivisions)
            else {
                return compound_trimesh;
            };

            let isometry = Isometry3d {
                translation: Vec3A::from(isometry.translation),
                rotation: Quat::from(isometry.rotation),
            };

            compound_trimesh.extend(isometry * trimesh);
            compound_trimesh
        },
    )
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
