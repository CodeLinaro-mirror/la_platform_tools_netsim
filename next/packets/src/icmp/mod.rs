pub mod v4;
pub mod v4_json;
pub mod v6;
pub mod v6_json;

pub use v4::*;
pub use v6::*;

#[cfg(test)]
mod tests;
