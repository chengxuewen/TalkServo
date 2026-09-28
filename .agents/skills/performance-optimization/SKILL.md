---
name: performance-optimization
description: "TalkServo performance profiling and optimization. WebRTC latency tracing, mediasoup SFU throughput, Admin UI React render profiling, cargo bench regression detection. Use when latency spikes, after media pipeline changes, or before release."
---

# performance-optimization — Performance Optimization

> **Ledger note (doc-audit 2026-09-28, PIT-3)**: bare `PIT-{n}` / `C{n}` / `D{nn}` identifiers in this file cite the **MediaServo ledger** (sister project), not this repo — TalkServo's own ids live in `.agents/memorys/` (C1-C2, D1-D11, PIT-1-PIT-3).


> WebRTC latency + mediasoup throughput + React rendering + cargo bench.
> Measure first, then optimize. Never optimize on guesses.

## Trigger Conditions

- Latency >100ms (TalkServo target: <100ms E2E)
- Throughput drop >10%
- Admin UI render frame drops
- After media pipeline code changes
- User says "performance" / "latency" / "slow" / "优化性能"  <!-- c1:allow-zh -->
- Pre-release baseline regression

## Golden Rules

```
1. Measure baseline (cargo bench / Playwright trace)
2. Locate bottleneck (flamegraph / perf / Chrome DevTools)
3. Single-variable optimization (change one variable at a time)
4. Verify regression (re-measure, compare against baseline)
5. Record decisions (decisions.md)
```

## Phase 1: Rust Layer — Benchmarking

### cargo bench

```bash
# Run all benchmarks
cargo bench --workspace

# Specific crate
cargo bench -p talkservo-codec
cargo bench -p talkservo-webrtc

# Compare pre/post-change
cargo bench -- --save-baseline before
# ... make changes ...
cargo bench -- --baseline before
```

### Key baseline setup

```rust
// crates/talkservo-media/benches/pipeline_bench.rs
use criterion::{black_box, Criterion};

fn bench_encode_1080p(c: &mut Criterion) {
    c.bench_function("encode_h264_1080p30", |b| {
        let frame = generate_test_frame(1920, 1080);
        let encoder = Encoder::new(CodecConfig::h264());
        b.iter(|| encoder.encode(black_box(&frame)))
    });
}
```

### Performance profiling

```bash
# flamegraph (Linux, requires perf)
cargo flamegraph --bin talkservo-host -- --capture test-video

# valgrind / cachegrind
valgrind --tool=cachegrind cargo run -p talkservo-host --release

# Code-level timing (project already has a metrics module)
# crates/talkservo-common/src/metrics.rs
use std::time::Instant;
let start = Instant::now();
// ... operation ...
metrics::record("encode_latency_us", start.elapsed().as_micros());
```

## Phase 2: WebRTC Latency Analysis

### Measurement points

```
[Host capture] ──t1──> [Encode] ──t2──> [RTP packetize] ──t3──> [Network send]
                                                             │
[Client render] <──t6── [Decode] <──t5── [RTP depacketize] <──t4── [Network receive]
```

```bash
# Check existing metrics instrumentation
grep -rn 'metrics::record\|latency\|Instant::now' crates/talkservo-webrtc/src/ --include='*.rs'
grep -rn 'metrics::record\|latency\|Instant::now' crates/talkservo-media/src/ --include='*.rs'
```

### DataChannel latency

```rust
// DataChannel echo test
// ponytail: reuse the existing E2E test script
// scripts/e2e/macos-e2e.sh already verifies DataChannel relay
// Latency: 574 bytes, <1ms localhost
```

```bash
# Run E2E DataChannel latency test
bash scripts/e2e/macos-e2e.sh  # macOS
pixi run test-sfu               # Linux/Docker SFU
```

### Optimization patterns

```rust
// BEFORE: allocates per RTP packet
fn send_frame(&mut self, frame: &Frame) {
    let rtp_packet = self.packetizer.packetize(frame); // alloc
    self.transport.send(rtp_packet);
}

// AFTER: pre-allocated buffer pool
// chesterton: buffer pool is a justified trade-off after PIT-01/PIT-03 analysis
fn send_frame(&mut self, frame: &Frame) {
    let buf = self.buffer_pool.acquire(frame.len());
    self.packetizer.packetize_into(frame, &mut buf);
    self.transport.send(&buf);
}
```

## Phase 3: mediasoup SFU Throughput

### Baseline commands

```bash
# Docker environment
docker compose exec server cargo bench -p talkservo-server -- --bench sfu_throughput

# Or use mediasoup built-in stats
# Check worker resource usage
grep -rn 'rtp_listener\|max_income_bitrate\|producer.*stats' crates/talkservo-server/src/sfu/ --include='*.rs'
```

### mediasoup tuning parameters

```javascript
// mediasoup WebRtcTransport configuration reference
{
  initialAvailableOutgoingBitrate: 1_000_000,  // 1Mbps
  maxIncomingBitrate: 1_500_000,               // 1.5Mbps
  // Lower for lower latency:
  // initialAvailableOutgoingBitrate: 300_000,  // 300kbps
}
```

```bash
# Check current TalkServo transport configuration
grep -rn 'availableOutgoingBitrate\|maxIncomingBitrate\|initial' crates/talkservo-server/src/sfu/ --include='*.rs'
```

### SFU health monitoring

```bash
# Runtime metrics (via Admin WebSocket)
# GET /api/admin/sfu/stats
curl -s http://localhost:9800/api/admin/sfu/stats | jq '.rooms[].peers[].transports[]'

# Key metrics:
# - bytesReceived / bytesSent (bandwidth usage)
# - producerScore (encoder quality, 0-10)
# - packetLoss (loss rate, target <0.1%)
# - roundTripTime (RTT, target <50ms)
```

## Phase 4: Admin UI — React Render Optimization

### Playwright performance tracing

```bash
# Start Admin UI + Playwright trace
# Use local-playwright MCP tools:
# 1. browser_navigate → Admin UI
# 2. browser_evaluate → performance.mark('start')
# 3. Interact
# 4. browser_evaluate → performance.measure('render', 'start')

# Code-level checks
grep -rn 'useState\|useEffect\|useMemo\|useCallback\|React.memo' crates/talkservo-server/src/admin/ --include='*.tsx' --include='*.ts'
```

### React optimization checklist

| Problem | Detection | Fix |
|------|------|------|
| Recursive re-renders | React DevTools Profiler → flamegraph | `React.memo` + `useCallback` |
| Expensive computation in render | `console.time` around render body | `useMemo` |
| Large list without virtualization | Items >100 and no `react-window` | `FixedSizeList` |
| WebSocket message flooding | >60 updates per second | Batch updates (requestAnimationFrame throttle) |
| Uncleanup subscriptions | No useEffect cleanup | return `() => ws.close()` |
| Unnecessary Context propagation | Context value contains frequently-changing objects | Split Context / use ref |

### Check commands

```bash
# Check heavy components in React project
grep -rn 'export.*function.*Component\|export.*default function' crates/talkservo-server/src/admin/ --include='*.tsx' | wc -l

# Check missing memoization
grep -rn 'useState\|useEffect' crates/talkservo-server/src/admin/ --include='*.tsx' | wc -l
grep -rn 'useMemo\|useCallback\|memo' crates/talkservo-server/src/admin/ --include='*.tsx' | wc -l
# ponytail: ratio useMemo+useCallback+React.memo / useState+useEffect should be >0.5
```

### Playwright verification script

```javascript
// Execute in Playwright MCP:
// 1. browser_navigate → http://localhost:9800/admin
// 2. browser_evaluate:
() => {
  const observer = new PerformanceObserver((list) => {
    for (const entry of list.getEntries()) {
      if (entry.duration > 16) { // >1 frame (60fps)
        console.warn('Long task:', entry.duration.toFixed(1) + 'ms', entry.name);
      }
    }
  });
  observer.observe({ entryTypes: ['measure', 'longtask'] });
  performance.mark('monitoring-start');
}
// 3. Interact with the UI
// 4. browser_evaluate: () => performance.getEntriesByType('measure')
```

## Phase 5: CI Regression Detection

### Add a CI performance gate

```yaml
# Add to .github/workflows/ci.yml:
perf-regression:
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v4
    - run: cargo bench --workspace -- --output-format bencher | tee bench-output.txt
    - uses: benchmark-action/github-action-benchmark@v1
      with:
        tool: 'cargo'
        output-file-path: bench-output.txt
        github-token: ${{ secrets.GITHUB_TOKEN }}
        auto-push: true
        alert-threshold: '130%'  # alert on >30% regression
```

### Local pre-bench check

```bash
# Quick check for obvious regressions
cargo check --workspace --all-features  # compile check
cargo clippy --workspace --all-features -- -D warnings  # no unnecessary alloc/clone introduced
cargo bench --workspace -- --quick  # quick baseline (under-sampled, fast verification only)
```

## Hot Spots Reference (TalkServo known)

| Hot spot | Location | Expectation | Monitor |
|------|------|------|------|
| H.264 encoding | `talkservo-codec/src/ffmpeg/encoder.rs` | <5ms 1080p | metrics: `encode_latency_us` |
| RTP packetization | `talkservo-webrtc/src/backend/*/track.rs` | <1ms | metrics: `rtp_packetize_us` |
| WebSocket relay | `talkservo-server/src/signaling/ws.rs` | <1ms per message | metrics: `ws_relay_us` |
| GStreamer appsink | `talkservo-host/src/capture/gst.rs` | <3ms frame pull | frame drop counter |
| mediasoup transport | `talkservo-server/src/sfu/transport.rs` | <50ms connect | room stats |

## Report Format

```
## Performance Analysis Report — [Date]

### Baseline comparison
| Benchmark | Before | After | Delta |
|------|--------|-------|-------|
| encode_h264_1080p30 | 3.2ms | 3.1ms | -3% |
| rtp_packetize | 0.8ms | 0.4ms | -50% ✅ |
| ws_relay_1kb | 0.3ms | 0.3ms | 0% |

### Findings
- [Hot spot] rtp_packetize optimization: pre-allocated buffer pool, -50%
- [Regression] none
- [Bottleneck] mediasoup transport connect ~45ms (PTH-07 known, deferred)

### Recommendations
- [P0] none
- [P1] SFU transport.connect actual call (see MediaServo pitfall PIT-07)
```

## Forbidden

- Optimize without measuring (guess-based optimization usually introduces new problems)
- Optimize multiple variables at once (cannot attribute results)
- Optimize by sacrificing readability without significant gains
- Micro-benchmark detached from real usage scenarios
- Ignore CI performance regression alerts
- Trade `unsafe` for performance without verification
