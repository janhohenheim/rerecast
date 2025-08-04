//! Serialization and deserialization of data for the editor integration.

use anyhow::Context as _;
use base64::prelude::*;
use bevy_ecs::prelude::*;
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;

/// Serializes a value to a JSON value in the format expected by the editor integration.
pub fn serialize<T: Serialize>(val: &T) -> Result<Value> {
    let bytes = bincode::serde::encode_to_vec(val, bincode::config::standard())?;

    /*
    let mut compression_encoder = ZlibEncoder::new(Vec::new(), Compression::fast());
    compression_encoder.write_all(&bytes)?;
    let bytes = compression_encoder.finish()?;
    */

    let string = BASE64_STANDARD.encode(bytes);

    Ok(Value::String(string))
}

/// Deserializes a JSON value in the format expected by the editor integration to a value.
pub fn deserialize<T: DeserializeOwned>(value: &Value) -> anyhow::Result<T> {
    let string = value.as_str().context("Expected a string")?;

    let bytes = BASE64_STANDARD.decode(string)?;

    /*
    let mut compression_decoder = ZlibDecoder::new(&bytes[..]);
    let mut bytes = Vec::new();
    compression_decoder.read_to_end(&mut bytes)?;
    */

    let (val, _len): (T, usize) =
        bincode::serde::decode_from_slice(&bytes, bincode::config::standard())?;
    Ok(val)
}
