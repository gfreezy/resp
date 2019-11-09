//! RESP(Redis Serialization Protocol) Serialization for Rust.

mod parser;
mod serialize;
mod value;

pub use value::{Error, Value};
