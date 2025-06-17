use avian_navmesh::prelude::*;
use bevy::{
    ecs::error::{GLOBAL_ERROR_HANDLER, warn},
    input::common_conditions::input_just_pressed,
    prelude::*,
    remote::{
        BrpRequest,
        builtin_methods::{BRP_QUERY_METHOD, BrpQuery, BrpQueryFilter, BrpQueryParams},
    },
};

fn main() -> AppExit {
    GLOBAL_ERROR_HANDLER
        .set(warn)
        .expect("The error handler can only be set once, globally.");

    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(NavMeshPlugin::default())
        .add_systems(
            Update,
            list_components.run_if(input_just_pressed(KeyCode::Space)),
        )
        .run()
}

/// The application entry point.
fn list_components() -> Result {
    // Create the URL. We're going to need it to issue the HTTP request.
    let host_part = format!("{}:{}", "127.0.0.1", 15702);
    let url = format!("http://{host_part}/");

    let req = BrpRequest {
        jsonrpc: String::from("2.0"),
        method: String::from(BRP_QUERY_METHOD),
        id: Some(serde_json::to_value(1)?),
        params: Some(
            serde_json::to_value(BrpQueryParams {
                data: BrpQuery {
                    components: vec![],
                    option: Vec::default(),
                    has: Vec::default(),
                },
                strict: false,
                filter: BrpQueryFilter::default(),
            })
            .expect("Unable to convert query parameters to a valid JSON value"),
        ),
    };

    let res = ureq::post(&url)
        .send_json(req)?
        .body_mut()
        .read_json::<serde_json::Value>()?;

    println!("{:#}", res);

    Ok(())
}
