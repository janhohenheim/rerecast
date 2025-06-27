#![doc = include_str!("../../../readme.md")]

mod serialization;
mod serialized_types;

pub use serialization::{deserialize, serialize};
pub use serialized_types::*;
