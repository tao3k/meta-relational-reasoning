//! Native AOT grammar binding interface.

mod driver;
#[allow(unsafe_code)]
mod ffi;
mod model;
mod reasoning;
mod runtime;

pub use driver::{
    DriverError, DriverPhase, DriverResource, DriverStatus, DriverTransition, driver_request,
    driver_transition,
};
pub(crate) use model::NativeGrammar;
pub use model::{
    FeatureDependencySpec, FeatureSpec, IsoProfile, IsoProfileLoadError, ModuleSpec,
    ProfileModuleSpec, ProfileSpec, ProfileSupplementSpec, ReleaseSpec, load_iso_profile,
};
pub use reasoning::{ReasoningBundleLoadError, load_reasoning_bundle};
