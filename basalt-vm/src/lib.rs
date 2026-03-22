#![allow(clippy::arc_with_non_send_sync)]

pub mod highlight;
pub mod value;
/// Basalt VM - Executes compiled bytecode.
pub mod vm;

pub use value::Value;
pub use vm::VM;
