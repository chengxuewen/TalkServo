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

Media engine for native clients = OQ-9, phased c→b; modules/02-06 server architecture is engine-agnostic by construction (floor/arbitration lives in core + WS wire; media is swappable under the client facade).

## Binding layout (Beta activation order, mirrors MediaServo's verified tree)

```text
bindings/
├── c/
│   ├── include/talkservo/client.h          # hand-maintained headers (D248 parity: no codegen promises)
│   ├── cmake/                              # FindTalkServo.cmake + pkg-config for C consumers
│   └── talkservo-client-c/                 # cdylib → libtalkservo_client.so, symbols `talkservo_*` (D247 parity)
├── cxx/
│   ├── include/talkservo/                  # header-only RAII over client.h (tl::expected-style Result vendored w/ NOTICE)
│   ├── talkservo-client-cxx/               # cargo member (builds the header check + examples)
│   └── examples/  tests/
├── python/talkservo/                       # ctypes package — NOT a cargo member (D228 parity); pyproject + sdist
└── mobile/                                 # UniFFI: .udl in talkservo-client, Kotlin/Swift shells generated per target app
```

Workspace: `-c`/`-cxx` crates are root `[workspace]` members (shared lock/toolchain, one `cargo build` builds all ABI artifacts); python/mobile dirs are out-of-members by design. Node: absent until a named consumer (D8). Binding CI jobs capped at 3 (C, Cxx, Py) — MediaServo's 4-way matrix is the cautionary precedent. ABI gates reused: symbol-prefix check, soname discipline, `cargo deny`, header compile test.

Node.js status update (D10, 2026-09-28): the Electron desktop shell names a *potential* napi consumer (main-process media/background hooks — deferred need, desktop media currently lives in the renderer's Chromium and needs no Rust binding). Rule unchanged: `bindings/node/` opens only when that need is real; the shell itself consumes `web/` + WS directly.
