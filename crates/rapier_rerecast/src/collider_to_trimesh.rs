//! Contains traits and methods for converting [`Collider`]s into trimeshes.

use bevy_math::{Quat, Vec3, Vec3A};
use bevy_rapier3d::{
    geometry::Collider,
    na::Isometry,
    parry::{
        shape::{Compound, TypedShape},
        transformation::utils,
    },
};
use bevy_rerecast_core::rerecast::{AreaType, TriMesh};

/// Convenience trait that allows a [`Collider`] to be converted into a [`TriMesh`].
pub trait ColliderToTriMesh {
    /// Converts the collider into a [`TriMesh`].
    ///
    /// # Arguments
    ///
    /// * `subdivisions` - The number of subdivisions to use for the collider. This is used for curved shapes such as circles and spheres.
    ///
    /// # Returns
    ///
    /// A [`TriMesh`] if the collider is supported, otherwise `None`
    ///
    /// The following shapes are not supported:
    /// - [`Segment`](bevy_rapier3d::parry::shape::Segment)
    /// - [`Polyline`](bevy_rapier3d::parry::shape::Polyline)
    /// - [`HalfSpace`](bevy_rapier3d::parry::shape::HalfSpace)
    /// - Custom shapes
    ///
    /// The following rounded shapes are supported, but only the inner shape without rounding is used:
    /// - [`RoundCuboid`](bevy_rapier3d::parry::shape::RoundCuboid)
    /// - [`RoundTriangle`](bevy_rapier3d::parry::shape::RoundTriangle)
    /// - [`RoundConvexPolyhedron`](bevy_rapier3d::parry::shape::RoundConvexPolyhedron)
    /// - [`RoundCylinder`](bevy_rapier3d::parry::shape::RoundCylinder)
    /// - [`RoundCone`](bevy_rapier3d::parry::shape::RoundCone)
    fn to_trimesh(
        &self,
        pos: impl Into<Vec3>,
        rot: impl Into<Quat>,
        subdivisions: u32,
    ) -> Option<TriMesh>;
}

impl ColliderToTriMesh for Collider {
    fn to_trimesh(
        &self,
        pos: impl Into<Vec3>,
        rot: impl Into<Quat>,
        subdivisions: u32,
    ) -> Option<TriMesh> {
        shape_to_trimesh(
            &self.as_unscaled_typed_shape().as_typed_shape(),
            pos.into(),
            rot.into(),
            subdivisions,
            self.scale(),
        )
    }
}

fn shape_to_trimesh(
    shape: &TypedShape,
    pos: Vec3,
    rot: Quat,
    subdivisions: u32,
    scale: Vec3,
) -> Option<TriMesh> {
    let (vertices, indices) = match shape {
        // Simple cases
        TypedShape::Cuboid(cuboid) => cuboid.to_trimesh(),
        TypedShape::Voxels(voxels) => voxels.to_trimesh(),
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
            return Some(compound_trimesh(compound, pos, rot, subdivisions, scale));
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
    let mut vertices = utils::scaled(vertices, scale.into());
    utils::transform(&mut vertices, Isometry::from_parts(pos.into(), rot.into()));

    let indices_len = indices.len();
    Some(TriMesh {
        vertices: vertices.into_iter().map(Vec3A::from).collect(),
        indices: indices.into_iter().map(|i| i.into()).collect(),
        area_types: vec![AreaType::NOT_WALKABLE; indices_len],
    })
}

fn compound_trimesh(
    compound: &Compound,
    pos: Vec3,
    rot: Quat,
    subdivisions: u32,
    scale: Vec3,
) -> TriMesh {
    compound.shapes().iter().fold(
        TriMesh::default(),
        |mut compound_trimesh, (sub_pos, shape)| {
            let pos = pos + rot.mul_vec3(sub_pos.translation.into());
            let rot = rot.mul_quat(sub_pos.rotation.into()).normalize();
            let Some(trimesh) =
                // No need to track recursive compounds because parry panics on nested compounds anyways lol
                shape_to_trimesh(&shape.as_typed_shape(), pos, rot, subdivisions, scale)
            else {
                return compound_trimesh;
            };

            compound_trimesh.extend(trimesh);
            compound_trimesh
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_math::{Quat, Vec3};
    use bevy_rerecast_core::rerecast::Aabb3d;

    const TEST_SCALE: f32 = 2.5;

    #[inline]
    fn get_typed_shape(collider: &Collider) -> TypedShape<'_> {
        collider.as_unscaled_typed_shape().as_typed_shape()
    }

    #[inline]
    fn create_trimesh_and_get_aabb(collider: &Collider) -> (TriMesh, Aabb3d) {
        let shape = get_typed_shape(collider);
        let trimesh = shape_to_trimesh(
            &shape,
            Vec3::ZERO,
            Quat::IDENTITY,
            16,
            Vec3::splat(TEST_SCALE),
        )
        .unwrap();
        let aabb = trimesh.compute_aabb().unwrap();
        (trimesh, aabb)
    }

    #[inline]
    fn test_aabb(name: &str, aabb: Aabb3d, expect_x: f32, expect_y: f32, expect_z: f32) {
        let got_x = aabb.max.x - aabb.min.x;
        let got_y = aabb.max.y - aabb.min.y;
        let got_z = aabb.max.z - aabb.min.z;

        assert!(
            (got_x - expect_x).abs() < f32::EPSILON,
            "{name} X should be ~{expect_x}, got {got_x}",
        );
        assert!(
            (got_y - expect_y).abs() < f32::EPSILON,
            "{name} Y should be ~{expect_y}, got {got_y}",
        );
        assert!(
            (got_z - expect_z).abs() < f32::EPSILON,
            "{name} Z should be ~{expect_z}, got {got_z}",
        );
    }

    #[inline]
    fn test_collider(name: &str, collider: Collider, expect_x: f32, expect_y: f32, expect_z: f32) {
        let (trimesh, aabb) = create_trimesh_and_get_aabb(&collider);
        println!("{trimesh:?}");
        println!("{aabb:?}");
        test_aabb(name, aabb, expect_x, expect_y, expect_z);
    }

    #[test]
    fn cuboid_aabb_size() {
        test_collider("Cuboid", Collider::cuboid(1.0, 2.0, 3.0), 5.0, 10.0, 15.0);
    }

    #[test]
    fn ball_aabb_size() {
        test_collider("Ball", Collider::ball(2.0), 10.0, 10.0, 10.0);
    }

    #[test]
    fn capsule_aabb_size() {
        test_collider("CapsuleY", Collider::capsule_y(2.0, 0.5), 2.5, 12.5, 2.5);
    }

    #[test]
    fn cylinder_aabb_size() {
        test_collider("Cylinder", Collider::cylinder(1.0, 0.5), 2.5, 5.0, 2.5);
    }

    #[test]
    fn cone_aabb_size() {
        test_collider("Cone", Collider::cone(1.0, 0.5), 2.5, 5.0, 2.5);
    }

    #[test]
    fn triangle_aabb_size() {
        let collider = Collider::triangle(
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        );
        test_collider("Triangle", collider, 2.5, 2.5, 0.0);
    }

    #[test]
    fn trimesh_aabb_size() {
        let vertices = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        ];
        let indices = vec![[0, 1, 2]];
        let collider = Collider::trimesh(vertices, indices).unwrap();
        test_collider("Trimesh", collider, 2.5, 2.5, 0.0);
    }

    #[test]
    fn heightfield_aabb_size() {
        let heights = vec![1.0, 2.0, 1.0, 0.0];
        let collider = Collider::heightfield(heights, 2, 2, Vec3::ONE);
        test_collider("Heightfield", collider, 2.5, 5.0, 2.5);
    }

    #[test]
    fn convex_polyhedron_aabb_size() {
        let vertices = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(0.0, 0.0, 1.0),
        ];
        let indices = vec![[0, 1, 2], [0, 2, 3], [0, 3, 1], [1, 3, 2]];
        let collider = Collider::convex_mesh(vertices, &indices).unwrap();
        test_collider("ConvexPolyhedron", collider, 2.5, 2.5, 2.5);
    }

    #[test]
    fn round_cuboid_aabb_size() {
        let collider = Collider::round_cuboid(0.5, 1.0, 1.5, 0.2);
        test_collider("RoundCuboid", collider, 2.5, 5.0, 7.5);
    }

    #[test]
    fn round_cylinder_aabb_size() {
        let collider = Collider::round_cylinder(1.0, 0.5, 0.1);
        test_collider("RoundCylinder", collider, 2.5, 5.0, 2.5);
    }

    #[test]
    fn round_cone_aabb_size() {
        let collider = Collider::round_cone(1.0, 0.5, 0.1);
        test_collider("RoundCone", collider, 2.5, 5.0, 2.5);
    }

    #[test]
    fn round_triangle_aabb_size() {
        let collider = Collider::round_triangle(
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
            0.1,
        );
        test_collider("RoundTriangle", collider, 2.5, 2.5, 0.0);
    }

    #[test]
    fn round_convex_polyhedron_aabb_size() {
        let vertices = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(0.0, 0.0, 1.0),
        ];
        let indices = vec![[0, 1, 2], [0, 2, 3], [0, 3, 1], [1, 3, 2]];
        let collider = Collider::round_convex_mesh(vertices, &indices, 0.1).unwrap();
        test_collider("RoundConvexPolyhedron", collider, 2.5, 2.5, 2.5);
    }

    #[test]
    fn compound_collider_aabb_size() {
        let cuboid = Collider::cuboid(1.0, 2.0, 3.0);
        let ball = Collider::ball(2.0);
        let collider = Collider::compound(vec![
            (Vec3::new(-10.0, 0.0, 5.0), Quat::IDENTITY, cuboid),
            (Vec3::new(0.0, -10.0, -5.0), Quat::IDENTITY, ball),
        ]);
        test_collider("Compound", collider, 17.5, 20.0, 22.5);
    }

    #[test]
    fn rasterizes_cuboid() {
        let collider = Collider::cuboid(1.0, 2.0, 3.0);
        let shape = get_typed_shape(&collider);
        let trimesh = shape_to_trimesh(&shape, Vec3::ZERO, Quat::IDENTITY, 1, Vec3::ONE).unwrap();
        assert_eq!(trimesh.vertices.len(), 8);
        assert_eq!(trimesh.indices.len(), 12);
    }
}
