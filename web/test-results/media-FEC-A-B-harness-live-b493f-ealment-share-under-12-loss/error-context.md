# Instructions

- Following Playwright test failed.
- Explain why, be concise, respect Playwright best practices.
- Provide a snippet of code with the fix, if possible.

# Test info

- Name: media.spec.ts >> FEC A/B harness (live host, netem) >> #8: FEC on reduces concealment-share under 12% loss
- Location: e2e/media.spec.ts:245:3

# Error details

```
Error: FEC-ON share 0.21169794188861984 must be below FEC-OFF share 0.1438467440619162

expect(received).toBeLessThan(expected)

Expected: < 0.1438467440619162
Received:   0.21169794188861984
```

# Test source

```ts
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
  235 |     expect(samples.length).toBeGreaterThanOrEqual(1);
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
  256 |       await joinField(listener, `${label}-listener`);
  257 |       await listener.waitForTimeout(3000);
  258 |       const ctxH = await browser.newContext();
  259 |       const holder = await ctxH.newPage();
  260 |       holder.on("console", (m) => { if (m.text().includes("FEC-DIAG")) console.log(`${label} ${m.text()}`); });
  261 |       const url = urlFor(`${label}-holder`) + (fec ? "" : "&fec=off");
  262 |       await holder.goto(url);
  263 |       await expect(holder.getByText("connected")).toBeVisible({ timeout: 15_000 });
  264 |       await holder.waitForTimeout(3000);
  265 | 
  266 |       await holder.keyboard.down("Space");
  267 |       await expect(holder.getByText(/HOLDING|SPEAKING/)).toBeVisible({ timeout: 10_000 });
  268 |       await listener.waitForTimeout(15_000);
  269 | 
  270 |       const stats = await listener.evaluate(async () => {
  271 |         const w = window as unknown as {
  272 |           __tsRecvT?: { getStats(): Promise<RTCStatsReport> };
  273 |         };
  274 |         if (!w.__tsRecvT) return null;
  275 |         const rep = await w.__tsRecvT.getStats();
  276 |         let concealed = 0;
  277 |         let total = 0;
  278 |         rep.forEach((rs) => {
  279 |           if (rs.type === "inbound-rtp" && rs.kind === "audio") {
  280 |             concealed = Number(rs.concealedSamples ?? 0);
  281 |             total = Number(rs.totalSamplesReceived ?? 0);
  282 |           }
  283 |         });
  284 |         return { concealed, total };
  285 |       });
  286 | 
  287 |       await holder.keyboard.up("Space");
  288 |       await ctxH.close();
  289 |       await ctxL.close();
  290 |       const share = stats && stats.total > 0 ? stats.concealed / stats.total : null;
  291 |       return { arm: label, fec, ...(stats ?? {}), share };
  292 |     };
  293 | 
  294 |     const a = await runArm("fecA", false);
  295 |     const b = await runArm("fecB", true);
  296 |     console.log(`FEC-AB ${JSON.stringify({ a, b })}`);
  297 | 
  298 |     expect(a.share, `A arm has no stats: ${JSON.stringify(a)}`).not.toBeNull();
  299 |     expect(b.share, `B arm has no stats: ${JSON.stringify(b)}`).not.toBeNull();
  300 |     // the acceptance claim: inband FEC lowers the concealment share
  301 |     expect(
  302 |       b.share!,
  303 |       `FEC-ON share ${b.share} must be below FEC-OFF share ${a.share}`,
> 304 |     ).toBeLessThan(a.share!);
      |       ^ Error: FEC-ON share 0.21169794188861984 must be below FEC-OFF share 0.1438467440619162
  305 |     // FEC must strictly reduce the concealment share under loss
  306 |     expect(b.share!).toBeLessThan(a.share!);
  307 |   });
  308 | });
  309 | 
```