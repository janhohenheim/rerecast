use bevy::prelude::*;
use bevy_rerecast::{NavmeshApp, rerecast::TriMesh};

pub(super) fn plugin(app: &mut App) {
    app.set_navmesh_affector_backend(editor_backend);
}

fn editor_backend() -> Vec<(GlobalTransform, TriMesh)> {
    let mut trimesh = TriMesh::default();
    for (mesh, transform) in affectors.iter() {
        let Some(mesh) = meshes.get(mesh) else {
            warn!("Failed to get mesh for navmesh build. Skipping.");
            continue;
        };
        let Some(mut current_trimesh) = TriMesh::from_mesh(mesh) else {
            warn!("Failed to convert collider to trimesh. Skipping.");
            continue;
        };
        let transform = transform.compute_transform();
        for vertex in &mut current_trimesh.vertices {
            *vertex = transform.transform_point(Vec3::from(*vertex)).into();
        }
        trimesh.extend(current_trimesh);
    }
}
