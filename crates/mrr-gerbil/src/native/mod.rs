//! Native AOT grammar binding interface.

#[allow(unsafe_code)]
mod ffi;
mod model;
mod reasoning;
mod runtime;

pub(crate) use model::NativeGrammar;
pub use reasoning::{ReasoningBundleLoadError, load_reasoning_bundle};
