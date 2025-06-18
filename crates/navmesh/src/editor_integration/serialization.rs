use anyhow::Context as _;
use base64::prelude::*;
use bevy::prelude::*;
use serde_json::Value;

use crate::editor_integration::input_data::{CloneProxy as _, ProxyMesh};

pub fn serialize_mesh(mesh: &Mesh) -> Result<Value> {
    let proxy_mesh = mesh.clone_proxy();
    let bincode_bytes = bincode::serde::encode_to_vec(&proxy_mesh, bincode::config::standard())?;
    let base64_string = BASE64_STANDARD.encode(bincode_bytes);
    Ok(Value::String(base64_string))
}

pub fn deserialize_mesh(value: &Value) -> Result<Mesh> {
    let value_string = value.as_str().context("Expected a string")?;
    let bincode_bytes = BASE64_STANDARD.decode(value_string)?;
    let (proxy_mesh, _len): (ProxyMesh, usize) =
        bincode::serde::decode_from_slice(&bincode_bytes, bincode::config::standard())?;
    let mesh = proxy_mesh.into();
    Ok(mesh)
}
