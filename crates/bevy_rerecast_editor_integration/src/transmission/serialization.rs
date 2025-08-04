//! Serialization and deserialization of data for the editor integration.

use std::{
    io::{Read as _, Write},
    time::Instant,
};

use anyhow::Context as _;
use base64::prelude::*;
use bevy_ecs::prelude::*;
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;

/// Serializes a value to a JSON value in the format expected by the editor integration.
pub fn serialize<T: Serialize>(val: &T) -> Result<Value> {
    let now = Instant::now();
    let bytes = bincode::serde::encode_to_vec(val, bincode::config::standard())?;
    println!("Serialization: {} ms", now.elapsed().as_millis());

    /*
    let now = Instant::now();
    let mut compression_encoder = ZlibEncoder::new(Vec::new(), Compression::fast());
    compression_encoder.write_all(&bytes)?;
    let bytes = compression_encoder.finish()?;
    println!("compression: {} ms", now.elapsed().as_millis());
    */

    let now = Instant::now();
    let string = BASE64_STANDARD.encode(bytes);
    println!("stringify: {} ms", now.elapsed().as_millis());

    Ok(Value::String(string))
}

/// Deserializes a JSON value in the format expected by the editor integration to a value.
pub fn deserialize<T: DeserializeOwned>(value: &Value) -> anyhow::Result<T> {
    let string = value.as_str().context("Expected a string")?;

    let now = Instant::now();
    let bytes = BASE64_STANDARD.decode(string)?;
    println!("unstringify: {} ms", now.elapsed().as_millis());

    /*
    let now = Instant::now();
    let mut compression_decoder = ZlibDecoder::new(&bytes[..]);
    let mut bytes = Vec::new();
    compression_decoder.read_to_end(&mut bytes)?;
    println!("decompression: {} ms", now.elapsed().as_millis()); */

    let now = Instant::now();
    let (val, _len): (T, usize) =
        bincode::serde::decode_from_slice(&bytes, bincode::config::standard())?;
    println!("deserialization: {} ms", now.elapsed().as_millis());
    Ok(val)
}
