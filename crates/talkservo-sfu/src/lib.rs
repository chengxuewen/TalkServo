//! talkservo-sfu — SFU host abstraction + backend selection (modules/03).
//!
//! Skeleton slice (plan-1): the `Sfu` trait shape and the compile-time
//! exactly-one-backend gate. Real mediasoup orchestration lands with plan-2.

/// Host-side media control surface. The signaling server drives the SFU
/// exclusively through this trait (modules/03 §Sfu).
pub trait Sfu {
    /// Human-readable backend name (observability label).
    fn backend(&self) -> &'static str;
}

// Exactly one media backend must be active (modules/03 feature block).
#[cfg(all(feature = "sfu-mediasoup", feature = "stub-media"))]
compile_error!("exactly one media backend must be active");

#[cfg(not(any(feature = "sfu-mediasoup", feature = "stub-media")))]
compile_error!("no media backend active");

#[cfg(feature = "sfu-mediasoup")]
mod mediasoup_backend {
    /// mediasoup-backed host (plan-2 wires the worker lifecycle).
    pub struct MediasoupSfu;

    impl super::Sfu for MediasoupSfu {
        fn backend(&self) -> &'static str {
            "mediasoup"
        }
    }
}
#[cfg(feature = "sfu-mediasoup")]
pub use mediasoup_backend::MediasoupSfu;

#[cfg(feature = "stub-media")]
mod stub_backend {
    /// No-op backend with canned responses — fake-WS test harness (plan-2 T-stub suite).
    #[derive(Default)]
    pub struct StubSfu;

    impl super::Sfu for StubSfu {
        fn backend(&self) -> &'static str {
            "stub"
        }
    }
}
#[cfg(feature = "stub-media")]
pub use stub_backend::StubSfu;

#[cfg(test)]
mod tests {
    #[test]
    fn default_backend_names_itself() {
        use crate::Sfu;
        #[cfg(feature = "sfu-mediasoup")]
        assert_eq!(crate::MediasoupSfu.backend(), "mediasoup");
        #[cfg(feature = "stub-media")]
        assert_eq!(crate::StubSfu.backend(), "stub");
    }
}
