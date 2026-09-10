//! Rust bindings to [Jeffrey Lim's JavelinPattern](https://github.com/jthlim/JavelinPattern).
//!
//! The [`bytes`] module matches byte slices and reports byte offsets. Compilation
//! is fallible; matching and scanning follow the native engine's semantics.
//!
//! ```
//! use javelin_pattern::bytes::Regex;
//!
//! let re = Regex::new(r"(\w+)=(\d+)")?;
//! let captures = re.captures(b"answer=42").unwrap();
//! assert_eq!(captures.get(1).unwrap().as_bytes(), b"answer");
//! assert_eq!(captures.get(2).unwrap().as_bytes(), b"42");
//! # Ok::<(), javelin_pattern::Error>(())
//! ```

#![deny(missing_docs, unsafe_op_in_unsafe_fn)]

pub mod bytes;
mod error;
mod ffi;

pub use error::Error;
