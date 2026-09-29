//! Stub internal state (kept here so `stub.rs` reads as the host surface).

use crate::host::ActivityState;
use std::collections::HashMap;

/// One peer's media handles in the stub world.
#[derive(Default, Debug, Clone)]
pub struct MediaPeer {
    pub connected: bool,
    /// producer_id → paused flag (created paused per D13).
    pub producers: HashMap<String, bool>,
    /// consumer_id → open flag.
    pub consumers: HashMap<String, bool>,
}

/// Whole-world state behind the stub's `Mutex`.
#[derive(Default, Debug, Clone)]
pub struct StubState {
    pub calls: Vec<crate::stub::Call>,
    /// room → peer → media
    pub rooms: HashMap<String, HashMap<String, MediaPeer>>,
    /// room → peer → scripted E11 answer
    pub activity: HashMap<String, HashMap<String, ActivityState>>,
    /// peers whose next create_transport fails (E4 script)
    pub fail_transport: Vec<String>,
    /// room → live transport count (E10 bookkeeping)
    pub transport_count: HashMap<String, usize>,
    pub kills: u32,
}
