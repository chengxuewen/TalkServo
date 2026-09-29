//! talkservo-core — the pure floor-domain crate.
//!
//! No async runtime, no I/O, no sockets (D5). Everything here is total,
//! deterministic-given-inputs logic over the floor state model (D12):
//! identity types, the wire contract (modules/02), and the `apply()`
//! reduction (rules 1-9) under exhaustive unit test.

pub mod ids;
pub mod wire;
