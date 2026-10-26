# SDK Strategy
> Design module of the TalkServo architecture family. Master doc: [../architecture.md](../architecture.md) — principles, open questions, acceptance. External evidence lives in [../reference/](../reference/); this folder holds design (what we build), reference holds knowledge (what exists). Convention C2.

## 1. SDK strategy (D8)

**Nothing is written in PoC** — the wire contract (modules/02 §1) is the SDK for browsers; native demand starts at Beta (mobile dispatch clients), not before.

Layer model (learned from MediaServo's four-SDK matrix and its D65→D222 churn; deliberately narrower):

```text
talkservo-client (Rust, single session facade: join / ptt / listen / floor-event callbacks)
      └─ C ABI base   (cdylib, prefix `talkservo_`, soname discipline, hand-maintained headers)
            ├─ C++ header-only RAII wrapper   (dispatch hardware's native tongue)
            ├─ Python ctypes thin layer        (bots / recording taps / test rigs)
            └─ UniFFI → Kotlin / Swift          (mobile; UniFFI itself emits a C ABI — same base, no fork)
      Node.js: NOT planned — no consumer identified (browser path free, server body is Rust).
      New language requires a named real consumer; binding CI jobs capped at 3 (C / Cxx / Py).
```

Rules inherited by reference and re-earned on use (MediaServo D227/240/241/247/248): single facade API (no facade matrices), C-ABI-is-contract, ABI stability + symbol prefix from day one of Beta, upgrade triggers before performance layers (ctypes→pyo3 equivalent: measured need only).

Native media engine (D15, OQ-9 closed): multi-backend compile-time dispatch, MediaServo D139-140 isomorph — `backend-webrtc-sys` (prebuilt libwebrtc FFI; LiveKit rust-sdks crate default candidate, sister vendored fork fallback) for production mobile; `backend-webrtc-rs` (pure-Rust secondary: desktop-Linux/CI; D3's engine revived here); `backend-stub`. The client crate defines one internal `ClientMediaBackend` trait (create/destroy/produce/consume/play + stats) — the mediasoup client protocol is written ONCE against it. Platform audio-session (AudioFocus/AVAudioSession) stays in the thin shells. webrtc-sys upstream (LiveKit vs sister fork) verified at Beta kickoff.

modules/02-06 server architecture is engine-agnostic by construction (floor/arbitration lives in core + WS wire; media is swappable under the client facade).

## Binding layout (Beta activation order, mirrors MediaServo's verified tree)

```text
bindings/
├── c/
│   ├── include/talkservo/client.h          # hand-maintained headers (MediaServo-sister D248 parity: no codegen promises)
│   ├── cmake/                              # FindTalkServo.cmake + pkg-config for C consumers
│   └── talkservo-client-c/                 # cdylib → libtalkservo_client.so, symbols `talkservo_*` (MediaServo-sister D247 parity)
├── cxx/
│   ├── include/talkservo/                  # header-only RAII over client.h (tl::expected-style Result vendored w/ NOTICE)
│   ├── talkservo-client-cxx/               # cargo member (builds the header check + examples)
│   └── examples/  tests/
├── python/talkservo/                       # ctypes package — NOT a cargo member (MediaServo-sister D228 parity); pyproject + sdist
└── mobile/                                 # UniFFI: .udl in talkservo-client, Kotlin/Swift shells generated per target app
```

Workspace: `-c`/`-cxx` crates are root `[workspace]` members (shared lock/toolchain, one `cargo build` builds all ABI artifacts); python/mobile dirs are out-of-members by design. Node: absent until a named consumer (D8). Binding CI jobs capped at 3 (C, Cxx, Py) — MediaServo's 4-way matrix is the cautionary precedent. ABI gates reused: symbol-prefix check, soname discipline, `cargo deny`, header compile test.

## TS client SDK (D14, 2026-09-28)

`packages/client` — same-language deliverable, orthogonal to the native binding matrix above. Facade: `new TalkServoClient({role}) → connect(url,jwt) / requestFloor({preempt}) / releaseFloor() / on('floor'|'peers'|'audio')`; internally owns WS lifecycle, floor-state mirror, mediasoup-client device/transport/produce/consume, R1 snapshot recovery, silent TokenRefresh, role-scoped snapshot dispatch (D12). Consumers: web SPA (thin views), Electron renderer (same package), Node bots (free — pure TS). PoC: in-repo only, unpublished; Beta: `@talkservo/client` npm publish, versioned with wire `v:1`. Wire types: hand-written TS unions mirroring modules/02 until `talkservo-core` emits the schema artifact.

Node.js status update (D10, 2026-09-28): the Electron desktop shell names a *potential* napi consumer (main-process media/background hooks — deferred need, desktop media currently lives in the renderer's Chromium and needs no Rust binding). Rule unchanged: `bindings/node/` opens only when that need is real; the shell itself consumes `web/` + WS directly.

## Binding matrix (final, 2026-09-28)

| Language | Channel | When | Consumer |
|----------|---------|------|----------|
| TS/JS | `@talkservo/client` (D14, wraps mediasoup-client) | PoC in-repo / Beta npm | SPA, Electron, Node bots (isomorphic, free) |
| Kotlin / Swift (mobile) | UniFFI over talkservo-client | Beta (with D15 backend) | field apps |
| C | cdylib C ABI (`talkservo_*`) | Beta | embedded / console base |
| C++ | header-only RAII over the same C ABI | Beta | dispatch-hardware OEMs |
| Python | ctypes over the same C ABI | Beta | bots / recording taps / probes |
| Node native (napi) | NOT planned — TS SDK already covers Node | trigger: main-process need TS cannot meet | — |
| C# | P/Invoke over the C ABI (zero new native work) | trigger-gated (WinForms/WPF dispatch seats, Unity) | none named yet |
| Java | JNI over the C ABI | trigger-gated | none named yet |

Rules: binding CI jobs capped at 3 (C/Cxx/Py); C#/Java compile against the .so/.dll and never enter the Rust build matrix; every new language requires a named real consumer first (D8).

## Delivery-face impact on the build graph (D28; planned at Beta, D8 trigger)

The runtime-deploy channel and the SDK-delivery channel stay SEPARATE — MediaServo's verified shape (`MediaServoBindings.cmake` + `package host|bindings` dual tar, D-H13) is the template, and the split is its load-bearing lesson:

- **Runtime deploy** = today's `talkservo-deploy-build` button: builds the configured `TALKSERVO_PACKAGE_LANE` handle and stages `out/<lane>/bin/{talkservo-server,oxmgr}`. It stages a runnable instance, nothing else. SDK artifacts must never ride this channel.
- **SDK delivery** = a future `cmake/TalkServoBindings.cmake` face (option-gated, opens only at the D8 trigger), mirroring the sister face point-for-point:
  - `polyorch_rust_build(SHARED)` for `bindings/c/talkservo-client-c` (ONE C-ABI cdylib — TalkServo has a single facade where MediaServo has four domains, so the matrix is 1 crate, not 4);
  - ONE `polyorch_rust_install(TARGETS talkservo-client-c SOVERSION VERSION STALE_GLOB "libtalkservo_*.so.[0-9]*" STRIP)` (co-located-install sweep semantics — single owner per swept destination);
  - one `install(DIRECTORY)` header fan-in into `include/talkservo/`;
  - configure-time `.pc` + `TalkServoConfig.cmake`/`ConfigVersion.cmake` renderers + a `bindings-gen` marker target;
  - staging (wheel/sdist/strip orchestration) stays out of cmake — python/ops own it, as in MediaServo.
- **Package contract (D-H13 parity)**: `dist/talkservo-server-<ver>.tar.gz` (today) + `dist/talkservo-sdk-<ver>.tar.gz` (Beta) — two tars, never one.
- **Open item at trigger time (cache groups)**: MediaServo's bindings lanes deliberately share the workspace `target/` because python staging and the ABI-drift check consume fixed paths; TalkServo's stub/live cache groups (`out/cargo-target/{stub,live}`, D27) conflict with that pattern. Decide at activation: bindings lane rides the stub group, or a third `out/cargo-target/bindings` group. Measure, do not guess (the .so-duplication failure mode is the documented reason both ways).
- **Cxx header-only + python ctypes are NOT cargo members / NOT lanes**: cxx is a header check + examples cargo member; python is a pyproject sdist (MediaServo D228 parity). Only `-c` gets a build button; UniFFI shells generate per app target, no lane of their own.
