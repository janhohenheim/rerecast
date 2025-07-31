//! Serialization and deserialization of data for the editor integration.

use anyhow::Context as _;
use base64::prelude::*;
use bevy_ecs::prelude::*;
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;

/// Serializes a value to a JSON value in the format expected by the editor integration.
pub fn serialize<T: Serialize>(val: &T) -> Result<Value> {
    let bincode_bytes = bincode::serde::encode_to_vec(val, bincode::config::standard())?;
    let base64_string = BASE64_STANDARD.encode(bincode_bytes);
    Ok(Value::String(base64_string))
}

/// Deserializes a JSON value in the format expected by the editor integration to a value.
pub fn deserialize<T: DeserializeOwned>(value: &Value) -> Result<T> {
    let value_string = value.as_str().context("Expected a string")?;
    let bincode_bytes = BASE64_STANDARD.decode(value_string)?;
    let (val, _len): (T, usize) =
        bincode::serde::decode_from_slice(&bincode_bytes, bincode::config::standard())?;
    Ok(val)
}
