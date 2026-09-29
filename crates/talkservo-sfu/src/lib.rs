//! talkservo-sfu — SFU host abstraction + backend selection (modules/03).
//!
//! Exactly one media backend active (compile-time gate below):
//! `sfu-mediasoup` (Linux, live worker) or `stub-media` (tests / macOS CI).

pub mod error;
pub mod host;
#[cfg(feature = "stub-media")]
pub mod stub;
#[cfg(feature = "stub-media")]
pub mod stub_internal;

pub use error::SfuError;
pub use host::{ActivityState, ProducerId, Sfu};
#[cfg(feature = "stub-media")]
pub use stub::StubSfu;
pub use talkservo_core::wire::TransportInfo;

// Exactly one media backend must be active (modules/03 feature block).
#[cfg(all(feature = "sfu-mediasoup", feature = "stub-media"))]
compile_error!("exactly one media backend must be active");

#[cfg(not(any(feature = "sfu-mediasoup", feature = "stub-media")))]
compile_error!("no media backend active");

#[cfg(feature = "sfu-mediasoup")]
mod mediasoup_host;

#[cfg(feature = "sfu-mediasoup")]
pub use mediasoup_host::{MediasoupSfu, Supervisor, WorkerRestarted};
