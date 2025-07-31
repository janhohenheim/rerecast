//! Types and functions needed for transmitting data between the editor and the running game.

mod serialization;
mod serialized_image;
mod serialized_mesh;
mod serialized_standard_material;

pub use serialization::*;
pub use serialized_image::*;
pub use serialized_mesh::*;
pub use serialized_standard_material::*;
