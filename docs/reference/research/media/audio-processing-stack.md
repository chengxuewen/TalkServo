# Audio Processing Stack — Opus FEC, NetEQ/Jitter, 3A Libraries

> **Status: FROZEN dossier (C2).** Point-in-time research; conclusions may be superseded (e.g. D6 engine change) — live truth is docs/architecture.md + docs/modules/. Do not back-write; new findings get new dated files.

> Compiled 2026-09-28 from the audio-stack-researcher session's verified fetches (RFC 6716/8215/7254/8310/8311 full texts; crates.io + api.github.com repo/crate status; webrtc.org + Chromium source docs), committed by the lead after the researcher session failed to write (PIT-1 pattern). Researcher-reported facts are cited; anything it could not confirm is marked UNVERIFIED.

## 1. Opus in-band FEC & loss handling

| Mechanism | Where specified | How it works | TalkServo implication |
|-----------|-----------------|--------------|---------------------|
| In-band FEC (SILK) | RFC 8215 (formerly 6716) §low-bitrate redundancy | current packet embeds a lower-quality encoding of the *previous* frame; enabled via `OPUS_SET_INBAND_FEC(1)`, most useful with VBR + DTX off/on per scenario | encoder init must set FEC; our manual pipeline owns detection of which frames have parity |
| FEC re-decode ("hidden decode") | RFC 8215 decoder API: decode with `fec=1` on a *lost* frame number | decoder reconstructs the lost frame from the redundancy carried in the *next* packet | receiver pipeline: on seq gap, when next packet arrives, run hidden decode for the missing frame before normal decode |
| PLC (packet loss concealment) | RFC 8215 §PLC | when no parity exists (FEC off or double loss), decoder extrapolates from history | automatic with libopus; quality degrades beyond ~1 lost frame — motivates FEC on |
| DTX + CNG | RFC 8215 §7 (discontinuous transmission + comfort noise) | silence suppression sends sparse CNF packets; interacts with FEC (FEC of a CNF frame is low-value) | PoC keeps DTX off (floor state already gates transmission); revisit for mobile power |

Packet-level loss patterns: WebRTC's own loss model (random + burst) applies; our PoC acceptance should test ~5-15% random loss with FEC on vs off (repeat of §9 scenario 2 under weak network).

## 2. NetEQ & jitter buffering

- **What NetEQ is** (webrtc.org audio receive side; Chromium `audio_coding/neteq`): an adaptive jitter buffer that does *time-stretching* (accelerate / preemptive expand), PLC-driven expansion, sample-level fade operations, and target-level adaptation from delay estimates — not a plain ring buffer.
- **What webrtc-rs gives us**: RTP seq/timestamp bookkeeping only; no NetEQ port. (Status note: a Rust NetEQ port does not exist as a maintained crate — UNVERIFIED completeness of this negative claim.)
- **Composed audio processing precedent**: Chromium's audio pipeline stacks APM (3A) before encode and NetEQ after decode — same shape as our D4-FFI split (APM on send side at capture; FEC/PLC on receive side).
- **PTT floor-churn specifics** (TalkServo-specific analysis, no upstream precedent found):
  - grant→revoke transitions truncate the buffer mid-stream: flush to zero on `FloorIdle`/`FloorTaken` *after* playing out the last granted packet, then ramp down, to avoid tail-of-talk clipping;
  - keep a small fixed target (40-60 ms, 2-3 frames) in PoC instead of adaptive logic — floor churn, not network jitter, dominates buffer pressure; upgrade to adaptive sizing in productization;
  - SRTP replay window (RFC 3711) may reject late retransmissions before NetEQ ever sees them — PoC does no retransmission (NACK), so this bites only if we add RTCP NACK later.

## 3. 3A library landscape (statuses verified 2026-09-28 via crates.io + api.github.com)

| Option | Crate/repo | Facts | Verdict |
|--------|-----------|-------|---------|
| **Google APM via FFI (D4, chosen)** | `webrtc-audio-processing` crate (sdlms/…; upstream chromium `modules/audio_processing`) | crate active with 2026-05 release (researcher-reported); C++ build dependency | PoC default: production-proven AEC/AGC/ANS; stub feature-gate for non-linux targets |
| AEC3 pure-Rust port | `aec3-rs` — **not a crates.io crate**; GitHub `xlinkica/aec3-rs` (Rust port of WebRTC AEC3, builds from C++ — treat as FFI-adjacent) | repo exists per GitHub search; maintenance level UNVERIFIED | productization candidate, not PoC |
| AEC3 Rust port (newer) | `webrtc-echo-canceller` | crates.io 0.2.1, released 2025-03-22 | watch-list; single-module scope |
| WebRTC3A pure-Rust attempt | `sonora` | no AEC scope (resample/dsp focus per upstream README) | **does not satisfy 3A** — corrects the planning-discussion assumption that sonora covers AEC |
| NS-only | `nnnoiseless` (RNNoise port) | crates.io 0.4.0, released 2022-10-28 — dated but functional | composable NS piece only |
| Legacy trio | speexdsp (AEC/AGC/ANS, superseded, low maintenance) | historical | skip |
| Browser-native (client-side free) | `getUserMedia` constraints `echoCancellation/noiseSuppression/autoGainControl` | W3C spec; per-OS/browser behavior differences; not fully disable-able on some Android builds | web client 3A for PoC comes free — the FFI path matters only for native SDK clients |
| Platform-native | Android `AcousticEchoCanceler` etc. | uneven device-level quality | fallback only |

**Conclusion**: D4 (FFI webrtc-audio-processing) is the only path with full 3A proven in production today; pure-Rust is a Beta+ evaluation with `aec3-rs`/`webrtc-echo-canceller` as candidates and `nnnoiseless` as a stopgap NS.

## 4. Recommended PoC pipeline (text diagram)

```text
CAPTURE (client)
  browser: getUserMedia(+browser 3A)      native: cpal + webrtc-audio-processing (FFI)
    -> Opus encode (48k VoIP, 20ms, VBR, FEC=on)
    -> webrtc-rs RTP pack -> SRTP/UDP ->
SFU (server)
  RTP parse (webrtc-rs srtp) -> relay decision from floor grant set (no decode) ->
RECEIVE (client)
  SRTP unprotect -> seq-gap detect -> hidden decode (fec=true) for lost frame
    -> [jitter buffer: fixed 2-3 frames, flush-on-release] -> Opus decode/PLC -> play
```

Risk annotations: (1) server-side SRTP unprotect/re-protect — PoC instead keeps SFU *relay-only* on SRTP (no crypto touch) — verify webrtc-rs supports that relay mode (`webrtc-rs/sfu` sans-IO crate verified: 82 stars, active (github-sweep-webrtc.md)); (2) browser 3A disable gaps (Android) — test matrix item; (3) jitter-buffer churn policy is ours to specify — §2 guidance is the starting spec.

## 5. Cross-doc notes

- Floor-arbitration precedents verified in this session: Mumble does per-packet talk-permission at relay (closer to our model than any conference system); LiveKit's revocable `canPublish` grants are the nearest modern API analogue to floor grant/revoke; neither is an *arbiter* with priority queue — MCPTT remains the formal benchmark (see standards-ptt-mcptt.md).
- `aurora-sfu` referenced in early discussion **does not exist** (0 GitHub hits, no crates.io crate) — removed from all docs (see oss-voice-infrastructure.md).

## Sources

- IETF RFCs 6716/8215 (Opus), 7254/8310/8311 (RTP/RTCP), fetched in full; NetEQ via webrtc.org docs & chromium.googlesource `modules/audio_coding/neteq`; 3A via `webrtc-audio-processing` crate page + upstream.
- crates.io API for `webrtc-audio-processing`, `webrtc-echo-canceller`, `nnnoiseless`, `sonora`; api.github.com for `xlinkica/aec3-rs`, `webrtc-rs/sfu`.
- Compiled from team session data 2026-09-28; UNVERIFIED items inline.
