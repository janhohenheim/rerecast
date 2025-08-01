//! Serialization and deserialization of data for the editor integration.

use std::io::{Read as _, Write};

use anyhow::Context as _;
use base64::prelude::*;
use bevy_ecs::prelude::*;
use flate2::{Compression, read::GzDecoder, write::GzEncoder};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;

/// Serializes a value to a JSON value in the format expected by the editor integration.
pub fn serialize<T: Serialize>(val: &T) -> Result<Value> {
    let bincode_bytes = bincode::serde::encode_to_vec(val, bincode::config::standard())?;
    let mut gz = GzEncoder::new(Vec::new(), Compression::default());
    gz.write_all(&bincode_bytes)?;
    let gz_bytes = gz.finish()?;
    let base64_string = BASE64_STANDARD.encode(gz_bytes);
    Ok(Value::String(base64_string))
}

/// Deserializes a JSON value in the format expected by the editor integration to a value.
pub fn deserialize<T: DeserializeOwned>(value: &Value) -> Result<T> {
    let value_string = value.as_str().context("Expected a string")?;
    let gz_bytes = BASE64_STANDARD.decode(value_string)?;

    let mut gz = GzDecoder::new(&gz_bytes[..]);
    let mut bincode_bytes = Vec::new();
    gz.read_to_end(&mut bincode_bytes)?;
    let (val, _len): (T, usize) =
        bincode::serde::decode_from_slice(&bincode_bytes, bincode::config::standard())?;
    Ok(val)
}
