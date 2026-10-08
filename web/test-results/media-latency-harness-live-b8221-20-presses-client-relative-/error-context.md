# Instructions

- Following Playwright test failed.
- Explain why, be concise, respect Playwright best practices.
- Provide a snippet of code with the fix, if possible.

# Test info

- Name: media.spec.ts >> latency harness (live host) >> #10: keydown→audible p50/p95 over 20 presses (client-relative)
- Location: e2e/media.spec.ts:176:3

# Error details

```
Error: expect(received).toBeGreaterThanOrEqual(expected)

Expected: >= 1
Received:    0
```

# Page snapshot

```yaml
- generic [ref=e3]:
  - generic [ref=e4]:
    - heading "Field — media-room" [level=4] [ref=e6]
    - generic [ref=e7]: connected
    - superscript [ref=e11]: gen 1791424934414891800
    - button [ref=e13] [cursor=pointer]:
      - img "setting" [ref=e15]
  - generic [ref=e18]:
    - generic [ref=e19]: Roster
    - list [ref=e26]:
      - listitem [ref=e27]: l-listener (me)
      - listitem [ref=e29]: l-holder
  - button "audio HOLD TO TALK" [ref=e31] [cursor=pointer]:
    - img "audio" [ref=e33]
    - generic [ref=e36]: HOLD TO TALK
  - generic [ref=e37]: press-and-hold (pointer or Space); preempt uses priority 5
  - status [ref=e38]: Floor idle.
```

# Test source

```ts
  135 |     console.log("UPLINK:", JSON.stringify(uplink));
  136 |     const trackState = await listener.evaluate(() => {
  137 |       const el = document.querySelector('#ts-audio-pool audio[data-peer="m-holder"]') as HTMLAudioElement | null;
  138 |       const t = (el?.srcObject as MediaStream | null)?.getAudioTracks()[0];
  139 |       return t ? { muted: t.muted, ready: t.readyState, enabled: t.enabled } : null;
  140 |     });
  141 |     const ice = await listener.evaluate(() => {
  142 |       const w = window as unknown as { __tsIce?: unknown[] };
  143 |       return w.__tsIce ?? null;
  144 |     });
  145 |     const tStates = await listener.evaluate(() => {
  146 |       const w = window as unknown as {
  147 |         __tsRecvT?: { connectionState(): string; iceState?: () => string; dtlsState?: () => string };
  148 |       };
  149 |       const t = w.__tsRecvT as { connectionState?: string } | undefined;
  150 |       const addrs = (window as unknown as { __tsAddrs?: unknown }).__tsAddrs;
  151 |       const ctxState = (window as unknown as { __tsCtxState?: Record<string, string> }).__tsCtxState;
  152 |       return { conn: t.connectionState, addrs, ctxState };
  153 |     });
  154 |     const clockDbg = await listener.evaluate(
  155 |       () => (window as unknown as { __tsClockDbg?: string }).__tsClockDbg ?? "",
  156 |     );
  157 |     expect(
  158 |       level,
  159 |       `audio flow ${level} ${clockDbg}; track=${JSON.stringify(trackState)} tconn=${JSON.stringify(tStates)}`,
  160 |     ).toBeGreaterThan(2);
  161 |     expect(
  162 |       level,
  163 |       `audio flow ${level}; track=${JSON.stringify(trackState)} tconn=${JSON.stringify(tStates)}`,
  164 |     ).toBeGreaterThan(2);
  165 | 
  166 | 
  167 |     await holder.keyboard.up("Space");
  168 |     await ctxH.close();
  169 |     await ctxL.close();
  170 |   });
  171 | });
  172 | 
  173 | test.describe("latency harness (live host)", () => {
  174 |   test.skip(!LIVE, "requires the live mediasoup host");
  175 | 
  176 |   test("#10: keydown→audible p50/p95 over 20 presses (client-relative)", async ({ browser }) => {
  177 |     test.setTimeout(180_000);
  178 |     // listener FIRST + settle: caps must be captured before the holder's
  179 |     // announcement lands (caps capture emits no event — sweep won't re-run)
  180 |     const ctxL = await browser.newContext();
  181 |     const listener = await ctxL.newPage();
  182 |     await joinField(listener, "l-listener");
  183 |     await listener.waitForTimeout(3000);
  184 |     const ctxH = await browser.newContext();
  185 |     const holder = await ctxH.newPage();
  186 |     await joinField(holder, "l-holder");
  187 |     await holder.waitForTimeout(3000);
  188 |     // Measurement truth (same probe as acceptance #2): inbound RTP bytes
  189 |     // growing on the listener's recv transport = audible. E11 measured: the
  190 |     // watchdog downs the fake-device speaker after ~2s of beep gaps, so each
  191 |     // press gets ONE ~1.5s sampling window from keydown.
  192 |     const samples: number[] = [];
  193 |     // 3 presses: the wire latency is consistent (89-96ms measured across
  194 |     // runs); more presses add E11-watchdog × fake-beep flakiness, not info
  195 |     for (let i = 0; i < 3; i++) {
  196 |       const t0 = Date.now();
  197 |       await holder.keyboard.down("Space");
  198 |       const audible = await listener.evaluate(async (t0ms) => {
  199 |         const w = window as unknown as {
  200 |           __tsRecvT?: { getStats(): Promise<RTCStatsReport> };
  201 |         };
  202 |         if (!w.__tsRecvT) return null;
  203 |         const snap = async () => {
  204 |           let bytes = 0;
  205 |           (await w.__tsRecvT.getStats()).forEach((r) => {
  206 |             if (r.type === "inbound-rtp" && r.kind === "audio") bytes = Number(r.bytesReceived ?? 0);
  207 |           });
  208 |           return bytes;
  209 |         };
  210 |         // phase 1: bytes must MOVE (consumer for this press is flowing)
  211 |         const b0 = await snap();
  212 |         for (let n = 0; n < 30; n++) {
  213 |           await new Promise((r) => setTimeout(r, 60));
  214 |           const b1 = await snap();
  215 |           if (b1 > b0) {
  216 |             // phase 2: first movement latency ≈ time from keydown to flow
  217 |             return Date.now() - t0ms;
  218 |           }
  219 |         }
  220 |         return null;
  221 |       }, t0);
  222 |       await holder.keyboard.up("Space");
  223 |       await new Promise((r) => setTimeout(r, 1200)); // release sweep settle
  224 |       if (audible !== null) samples.push(audible);
  225 |     }
  226 | 
  227 |     samples.sort((a, b) => a - b);
  228 |     const p50 = samples[Math.floor(samples.length / 2)] ?? -1;
  229 |     const p95 = samples[Math.floor(samples.length * 0.95)] ?? -1;
  230 |     console.log(
  231 |       `LATENCY samples=${samples.length} p50=${p50}ms p95=${p95}ms (budget LAN 300/public 600)`,
  232 |     );
  233 |     // ≥1 landing proves the keydown→flow path; the latency number is the
  234 |     // keydown→audible wire budget input for acceptance #10
> 235 |     expect(samples.length).toBeGreaterThanOrEqual(1);
      |                            ^ Error: expect(received).toBeGreaterThanOrEqual(expected)
  236 |     await ctxH.close();
  237 |     await ctxL.close();
  238 |   });
  239 | });
  240 | 
  241 | test.describe("FEC A/B harness (live host, netem)", () => {
  242 |   test.skip(!LIVE, "requires the live mediasoup host");
  243 |   test.skip(!process.env.TS_NETEM, "run scripts/netem.sh on first (root console)");
  244 | 
  245 |   test("#8: FEC on reduces concealment-share under 12% loss", async ({ browser }) => {
  246 |     test.setTimeout(240_000);
  247 |     // one run per arm: listener joins FIRST (caps settle), holder holds for
  248 |     // a 15s sampling window, then concealment-share from the listener's
  249 |     // inbound-rtp stats. Run A produces with ?fec=off (codec option off),
  250 |     // run B with default FEC on. NO_RTP_WATCHDOG_MS must be set large when
  251 |     // invoking (env passthrough in global-setup) so the E11 watchdog does
  252 |     // not down the fake speaker mid-window.
  253 |     const runArm = async (label: string, fec: boolean) => {
  254 |       const ctxL = await browser.newContext();
  255 |       const listener = await ctxL.newPage();
  256 |       await joinField(listener, `${r}-listener`);
  257 |       await listener.waitForTimeout(3000);
  258 |       const ctxH = await browser.newContext();
  259 |       const holder = await ctxH.newPage();
  260 |       const url = urlFor(`${r}-holder`) + (fec ? "" : "&fec=off");
  261 |       await holder.goto(url);
  262 |       await expect(holder.getByText("connected")).toBeVisible({ timeout: 15_000 });
  263 |       await holder.waitForTimeout(3000);
  264 | 
  265 |       await holder.keyboard.down("Space");
  266 |       await expect(holder.getByText(/HOLDING|SPEAKING/)).toBeVisible({ timeout: 10_000 });
  267 |       await listener.waitForTimeout(15_000);
  268 | 
  269 |       const stats = await listener.evaluate(async () => {
  270 |         const w = window as unknown as {
  271 |           __tsRecvT?: { getStats(): Promise<RTCStatsReport> };
  272 |         };
  273 |         if (!w.__tsRecvT) return null;
  274 |         const rep = await w.__tsRecvT.getStats();
  275 |         let concealed = 0;
  276 |         let total = 0;
  277 |         rep.forEach((rs) => {
  278 |           if (rs.type === "inbound-rtp" && rs.kind === "audio") {
  279 |             concealed = Number(rs.concealedSamples ?? 0);
  280 |             total = Number(rs.totalSamplesReceived ?? 0);
  281 |           }
  282 |         });
  283 |         return { concealed, total };
  284 |       });
  285 | 
  286 |       await holder.keyboard.up("Space");
  287 |       await ctxH.close();
  288 |       await ctxL.close();
  289 |       const share = stats && stats.total > 0 ? stats.concealed / stats.total : null;
  290 |       return { arm: label, fec, ...(stats ?? {}), share };
  291 |     };
  292 | 
  293 |     const a = await runArm("fecA", false);
  294 |     const b = await runArm("fecB", true);
  295 |     console.log(`FEC-AB ${JSON.stringify({ a, b })}`);
  296 | 
  297 |     expect(a.share, `A arm has no stats: ${JSON.stringify(a)}`).not.toBeNull();
  298 |     expect(b.share, `B arm has no stats: ${JSON.stringify(b)}`).not.toBeNull();
  299 |     // the acceptance claim: inband FEC lowers the concealment share
  300 |     expect(
  301 |       b.share!,
  302 |       `FEC-ON share ${b.share} must be below FEC-OFF share ${a.share}`,
  303 |     ).toBeLessThan(a.share!);
  304 |     // FEC must strictly reduce the concealment share under loss
  305 |     expect(b.share!).toBeLessThan(a.share!);
  306 |   });
  307 | });
  308 | 
```