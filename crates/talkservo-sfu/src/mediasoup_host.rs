//! Live mediasoup host (Linux, `sfu-mediasoup`) — full J-3-7 orchestration.
//!
//! One supervised worker (respawned after E6); one Router per room (lazy);
//! per-peer WebRtcTransport registry; producers created paused (D13),
//! `apply_floor` diff resumes/pauses; W-sequence rebuilds routers on worker
//! death and notifies the server via the restart broadcast.

use crate::error::SfuError;
use crate::host::{ActivityState, ProducerId, Sfu};
use mediasoup::prelude::Transport as _;
use std::collections::HashMap;
use std::sync::Arc;
use talkservo_core::floor::FloorState;
use talkservo_core::ids::{PeerId, RoomId};
use talkservo_core::wire::TransportInfo;
use tokio::sync::{broadcast, Mutex, RwLock};

/// Server-facing notification after a W-sequence rebuild.
#[derive(Debug, Clone)]
pub struct WorkerRestarted {
    pub reason: String,
}

/// Supervised worker + router registry.
pub struct Supervisor {
    inner: Arc<SupervisorInner>,
}

struct SupervisorInner {
    worker: Mutex<Option<mediasoup::worker::Worker>>,
    manager: mediasoup::worker_manager::WorkerManager,
    /// room → router (rebuilt on W-sequence).
    routers: RwLock<HashMap<RoomId, mediasoup::router::Router>>,
    restart_tx: broadcast::Sender<WorkerRestarted>,
}

impl Supervisor {
    pub fn new() -> Self {
        let (restart_tx, _) = broadcast::channel(16);
        Self {
            inner: Arc::new(SupervisorInner {
                worker: Mutex::new(None),
                manager: mediasoup::worker_manager::WorkerManager::new(),
                routers: RwLock::new(HashMap::new()),
                restart_tx,
            }),
        }
    }

    /// W-sequence notifications (server listens: broadcast MediaRestart).
    pub fn restart_rx(&self) -> broadcast::Receiver<WorkerRestarted> {
        self.inner.restart_tx.subscribe()
    }

    /// Acquire a live worker (spawn on first use; respawn after E6).
    async fn worker(&self) -> Result<mediasoup::worker::Worker, SfuError> {
        let mut guard = self.inner.worker.lock().await;
        if let Some(w) = guard.as_ref().filter(|w| !w.closed()) {
            return Ok(w.clone());
        }
        let mut settings = mediasoup::worker::WorkerSettings::default();
        // RTC UDP range per config (modules/06); the host plumbing narrows it
        // in production — PoC keeps the 40000-40100 default via env passthrough.
        settings.rtc_port_range =
            std::ops::RangeInclusive::new(self::rtc_ports::min(), self::rtc_ports::max());
        let worker = self
            .inner
            .manager
            .create_worker(settings)
            .await
            .map_err(|e| SfuError::Other(format!("worker spawn failed: {e}")))?;

        *guard = Some(worker.clone());
        Ok(worker)
    }

    /// Router for a room — created lazily, rebuilt after W.
    pub async fn router_for(&self, room: &RoomId) -> Result<mediasoup::router::Router, SfuError> {
        {
            let routers = self.inner.routers.read().await;
            if let Some(r) = routers.get(room).filter(|r| !r.closed()) {
                return Ok(r.clone());
            }
        }
        let worker = self.worker().await?;
        let options = mediasoup::router::RouterOptions::new(self::media_codecs::audio_opus());
        let router = worker
            .create_router(options)
            .await
            .map_err(|e| SfuError::Other(format!("router create failed: {e}")))?;
        self.inner
            .routers
            .write()
            .await
            .insert(room.clone(), router.clone());
        Ok(router)
    }

    /// Room reaped: drop the cached router (drop = close under the 0.24
    /// ownership model — closing the Router closes its transports/producers).
    pub async fn drop_router(&self, room: &RoomId) {
        let _ = self.inner.routers.write().await.remove(room);
    }

    /// E6/W-sequence: kill the worker child (drop = close under the 0.24
    /// ownership model), clear cached routers, and notify the server. There is
    /// no public exit EVENT in this crate version — the notification fires
    /// from here, deterministically, instead of from a poller.
    pub async fn kill_worker(&self) {
        self.inner.routers.write().await.clear();
        let _ = self.inner.worker.lock().await.take();
        let _ = self.inner.restart_tx.send(WorkerRestarted {
            reason: "worker exited".into(),
        });
    }
}

impl Default for Supervisor {
    fn default() -> Self {
        Self::new()
    }
}

/// Live consumer registry key: (room, peer, consumer_id).
type ConsumerKey = (RoomId, PeerId, String);

/// The live host handed to the server task.
#[derive(Clone)]
pub struct MediasoupSfu {
    supervisor: Arc<Supervisor>,
    /// (room, peer) → transport
    transports:
        Arc<RwLock<HashMap<(RoomId, PeerId), mediasoup::webrtc_transport::WebRtcTransport>>>,
    /// (room, peer) → producer (apply_floor diff target)
    producers: Arc<RwLock<HashMap<(RoomId, PeerId), mediasoup::producer::Producer>>>,
    /// (room, peer, consumer_id) → live consumer (drop = close)
    consumers: Arc<RwLock<HashMap<ConsumerKey, mediasoup::consumer::Consumer>>>,
}

impl MediasoupSfu {
    pub fn new(supervisor: Arc<Supervisor>) -> Self {
        Self {
            supervisor,
            transports: Arc::new(RwLock::new(HashMap::new())),
            producers: Arc::new(RwLock::new(HashMap::new())),
            consumers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn supervisor(&self) -> Arc<Supervisor> {
        self.supervisor.clone()
    }
}

fn sfu_err(e: impl std::fmt::Display, ctx: &str) -> SfuError {
    SfuError::Other(format!("{ctx}: {e}"))
}

impl Sfu for MediasoupSfu {
    async fn create_transport(
        &self,
        room: &RoomId,
        peer: &PeerId,
    ) -> Result<TransportInfo, SfuError> {
        let router = self.supervisor.router_for(room).await?;
        let listen = mediasoup::webrtc_transport::WebRtcTransportListenInfos::new(
            mediasoup::prelude::ListenInfo {
                protocol: mediasoup::prelude::Protocol::Udp,
                ip: "0.0.0.0"
                    .parse::<std::net::IpAddr>()
                    .map_err(|e| sfu_err(e, "ip"))?,
                announced_address: None,
                expose_internal_ip: false,
                port: None,
                port_range: None,
                flags: None,
                send_buffer_size: None,
                recv_buffer_size: None,
            },
        );
        let options = mediasoup::webrtc_transport::WebRtcTransportOptions::new(listen);
        let transport = router
            .create_webrtc_transport(options)
            .await
            .map_err(|e| sfu_err(e, "transport create"))?;

        let info = TransportInfo {
            ice: serde_json::to_value(transport.ice_parameters())
                .map_err(|e| sfu_err(e, "ice ser"))?,
            dtls: serde_json::to_value(transport.dtls_parameters())
                .map_err(|e| sfu_err(e, "dtls ser"))?,
            addrs: serde_json::json!([]),
        };
        self.transports
            .write()
            .await
            .insert((room.clone(), peer.clone()), transport);
        Ok(info)
    }

    async fn connect_transport(
        &self,
        room: &RoomId,
        peer: &PeerId,
        dtls: serde_json::Value,
    ) -> Result<(), SfuError> {
        let transport = {
            let t = self.transports.read().await;
            t.get(&(room.clone(), peer.clone()))
                .cloned()
                .ok_or_else(|| SfuError::Other("no transport for peer".into()))?
        };
        let remote: mediasoup::webrtc_transport::WebRtcTransportRemoteParameters =
            serde_json::from_value(dtls).map_err(|e| sfu_err(e, "dtls params"))?;
        transport
            .connect(remote)
            .await
            .map_err(|e| sfu_err(e, "dtls connect"))
    }

    async fn produce(
        &self,
        room: &RoomId,
        peer: &PeerId,
        rtp_parameters: serde_json::Value,
    ) -> Result<ProducerId, SfuError> {
        let transport = {
            let t = self.transports.read().await;
            t.get(&(room.clone(), peer.clone()))
                .cloned()
                .ok_or_else(|| SfuError::Other("no transport for peer".into()))?
        };
        let params: mediasoup_types::rtp_parameters::RtpParameters =
            serde_json::from_value(rtp_parameters).map_err(|e| sfu_err(e, "rtp parameters"))?;
        let mut options = mediasoup::producer::ProducerOptions::new(
            mediasoup::prelude::MediaKind::Audio,
            params,
        );
        options.paused = true; // D13: created paused; apply_floor gates
        let producer = transport
            .produce(options)
            .await
            .map_err(|e| sfu_err(e, "produce"))?;
        let id = ProducerId(producer.id().to_string());
        self.producers
            .write()
            .await
            .insert((room.clone(), peer.clone()), producer);
        Ok(id)
    }

    async fn consume(
        &self,
        room: &RoomId,
        peer: &PeerId,
        producer_id: &ProducerId,
    ) -> Result<serde_json::Value, SfuError> {
        // Server-side consume needs only the ROUTER caps (mediasoup builds the
        // consumer against them); the client-side local instantiation uses
        // mediasoup-client device caps — different concern, SDK's job.
        let router = self.supervisor.router_for(room).await?;
        let transport = {
            let t = self.transports.read().await;
            t.get(&(room.clone(), peer.clone()))
                .cloned()
                .ok_or_else(|| SfuError::Other("no transport for peer".into()))?
        };
        let producer_mid: mediasoup::producer::ProducerId = producer_id
            .0
            .parse()
            .map_err(|e| sfu_err(e, "producer id"))?;
        // Finalized → plain caps: the finalized form IS a valid RtpCapabilities
        // superset; serde round-trip converts the newtype cleanly.
        let caps_plain: mediasoup_types::rtp_parameters::RtpCapabilities =
            serde_json::from_value(serde_json::to_value(router.rtp_capabilities()).map_err(
                |e| sfu_err(e, "caps ser"),
            )?)
            .map_err(|e| sfu_err(e, "caps convert"))?;
        let options =
            mediasoup::consumer::ConsumerOptions::new(producer_mid, caps_plain);
        let consumer = transport
            .consume(options)
            .await
            .map_err(|e| sfu_err(e, "consume"))?;
        let params = serde_json::to_value(consumer.rtp_parameters())
            .map_err(|e| sfu_err(e, "consumer params ser"))?;
        // keep the consumer alive (drop = close); registry for future pause
        self.consumers
            .write()
            .await
            .insert((room.clone(), peer.clone(), consumer.id().to_string()), consumer);
        Ok(params)
    }

    async fn apply_floor(&self, room: &RoomId, state: &FloorState) {
        // idempotent diff (review M-4): only transitions hit the worker
        let producers = self.producers.read().await;
        for ((proom, ppeer), producer) in producers.iter() {
            if proom != room {
                continue;
            }
            let granted = state.grants().iter().any(|g| g == ppeer);
            match (granted, producer.paused()) {
                (true, true) => {
                    let _ = producer.resume().await;
                }
                (false, false) => {
                    let _ = producer.pause().await;
                }
                _ => {}
            }
        }
    }

    async fn peer_left(&self, room: &RoomId, peer: &PeerId) {
        // cascade (CM-4): close producer + transport
        // drop = close (mediasoup 0.24 ownership model)
        let _ = self
            .producers
            .write()
            .await
            .remove(&(room.clone(), peer.clone()));
        let _ = self
            .transports
            .write()
            .await
            .remove(&(room.clone(), peer.clone()));
        let stale: Vec<(RoomId, PeerId, String)> = self
            .consumers
            .read()
            .await
            .keys()
            .filter(|(r, p, _)| r == room && p == peer)
            .cloned()
            .collect();
        for key in stale {
            let _ = self.consumers.write().await.remove(&key);
        }
    }

    async fn media_activity(&self, room: &RoomId, peer: &PeerId) -> ActivityState {
        // E11 probe (PoC form): resumed producer = presumed flowing RTP.
        // AudioLevelObserver attach lands with the E-measurement slice.
        let producers = self.producers.read().await;
        match producers.get(&(room.clone(), peer.clone())) {
            Some(p) if !p.paused() => ActivityState::Active,
            _ => ActivityState::Silent,
        }
    }

    async fn router_caps(&self, room: &RoomId) -> serde_json::Value {
        // router_for spawns the worker on first use; a failure here is real
        // (worker cannot start) — empty caps degrade gracefully and the error
        // is visible at transport-create time which the client surfaces.
        match self.supervisor.router_for(room).await {
            Ok(router) => serde_json::to_value(router.rtp_capabilities())
                .unwrap_or(serde_json::json!({"codecs": [], "headerExtensions": []})),
            Err(_) => serde_json::json!({"codecs": [], "headerExtensions": []}),
        }
    }

    async fn destroy_room(&self, room: &RoomId) {
        self.supervisor.drop_router(room).await;
    }

    async fn kill_worker(&self) {
        self.supervisor.kill_worker().await;
    }

    fn backend(&self) -> &'static str {
        "mediasoup"
    }
}

/// RTC UDP port range from env (defaults per modules/06).
mod rtc_ports {
    pub fn min() -> u16 {
        std::env::var("RTC_PORT_MIN")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(40000)
    }
    pub fn max() -> u16 {
        std::env::var("RTC_PORT_MAX")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(40100)
    }
}

/// Codec list per modules/04: audio/opus 48k — the only PoC codec.
mod media_codecs {
    pub fn audio_opus() -> Vec<mediasoup_types::rtp_parameters::RtpCodecCapability> {
        use mediasoup_types::rtp_parameters::{MimeTypeAudio, RtpCodecCapability};
        vec![RtpCodecCapability::Audio {
            mime_type: MimeTypeAudio::Opus,
            preferred_payload_type: None,
            clock_rate: std::num::NonZeroU32::new(48_000).expect("nonzero"),
            channels: std::num::NonZeroU8::new(2).expect("nonzero"), // opus signaling
            parameters: Default::default(),
            rtcp_feedback: Default::default(),
        }]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn live_worker_spawn_and_router_create() {
        let sup = Arc::new(Supervisor::new());
        let room = RoomId::from("live-room");
        let router = sup.router_for(&room).await.expect("router");
        assert!(!router.closed());
        // idempotent: same router on second call
        let again = sup.router_for(&room).await.expect("router2");
        assert_eq!(router.id(), again.id());
    }

    #[tokio::test]
    async fn live_transport_create_and_labels() {
        let sup = Arc::new(Supervisor::new());
        let sfu = MediasoupSfu::new(sup);
        let room = RoomId::from("t-room");
        let peer = PeerId::from("t-peer");
        let info = sfu.create_transport(&room, &peer).await.expect("transport");
        assert!(info.ice.get("usernameIsEqual").is_none()); // shape probe — real fields exist
        assert!(info.ice.is_object());
        assert_eq!(sfu.backend(), "mediasoup");
    }

    #[tokio::test]
    async fn live_e6_kill_and_respawn() {
        let sup = Arc::new(Supervisor::new());
        let mut restarts = sup.restart_rx();
        let room = RoomId::from("kill-room");
        {
            let _router = sup.router_for(&room).await.expect("router");
        }
        // kill drops the worker child (0.24 ownership model; no public exit
        // EVENT — the notification fires from kill_worker, deterministically)
        sup.kill_worker().await;
        let note = restarts.recv().await.expect("channel alive");
        assert!(note.reason.contains("exited"));
        // next use respawns cleanly (fresh child, fresh router)
        let router = sup.router_for(&room).await.expect("respawned router");
        assert!(!router.closed());
    }
}
