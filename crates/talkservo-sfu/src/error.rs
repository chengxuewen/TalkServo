//! SFU host errors (thiserror, plan-2 T1).

#[derive(Debug, thiserror::Error)]
pub enum SfuError {
    /// E4: transport create/connect exceeded the mediasoup 10 s window.
    #[error("media transport timeout for peer {peer}")]
    TransportTimeout { peer: String },
    /// E10: transport guardrail / port pool exhausted.
    #[error("media capacity exhausted: {0}")]
    Capacity(String),
    /// E6: worker died mid-operation (W-sequence will rebuild).
    #[error("mediasoup worker gone")]
    WorkerGone,
    /// DTLS connect rejected / bad parameters.
    #[error("dtls failure: {0}")]
    Dtls(String),
    /// Anything else (worker request errors carry a reason string).
    #[error("sfu backend error: {0}")]
    Other(String),
}
