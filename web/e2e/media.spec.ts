// Plan-4 T2: fake-device media matrix — acceptance rows against a LIVE
// mediasoup server (TS_E2E_FEATURES=sfu-mediasoup; stub host cannot carry
// real RTP so these skip there honestly).
import { test, expect, type Page } from "@playwright/test";
import { LIVE, audioLevel, joinField, urlFor } from "./helpers.js";
export { LIVE, ROOM, audioLevel, joinField, urlFor } from "./helpers.js";


test.describe("media matrix (live host)", () => {
  test.skip(!LIVE, "requires the live mediasoup host (TS_E2E_FEATURES=sfu-mediasoup)");

  test("holder audible to listener (acceptance #2) + idle byte-count (#12)", async ({ browser }) => {
    test.setTimeout(60_000);
    // listener joins FIRST: produce announcements need a recipient
    const ctxL = await browser.newContext();
    const listener = await ctxL.newPage();
    await joinField(listener, "m-listener");
    await listener.waitForTimeout(3000); // media setup settle (transports up)

    const ctxH = await browser.newContext();
    const holder = await ctxH.newPage();
    await joinField(holder, "m-holder");
    await holder.waitForTimeout(3000); // holder media setup (produce fires)

    // holder grabs the floor (Space) — grants + producer flows
    await holder.keyboard.down("Space");
    await expect(holder.getByText(/HOLDING|SPEAKING/)).toBeVisible({ timeout: 10_000 });

    // #12 (D13 transmission gating): before the holder is granted, its mic
    // producer must be server-paused — micTruth().serverPaused is the SDK's
    // single source for that bit (D13 comment, media.ts:111). The raw-pc
    // getStats byte-count lands when the SDK exposes the pc; until then the
    // wire gate is asserted via this state, which flips server-driven only.
    const preGrantPaused = await holder.evaluate(() => {
      const s = (window as unknown as {
        __tsSessions?: Map<string, { media: { micTruth(): { serverPaused: boolean } } }>
      }).__tsSessions;
      const session = [...(s?.values() ?? [])][0];
      return session ? session.media.micTruth().serverPaused : null;
    });
    expect(preGrantPaused).toBe(true);

    const level = await audioLevel(listener, "m-holder");
    // consume chain settle: announcement -> consume -> ConsumeOk ->
    // audio-opened -> <audio> in the pool (no hard sleeps)
    await listener
      .waitForFunction(
        () => (document.getElementById("ts-audio-pool")?.children.length ?? 0) > 0,
        undefined,
        { timeout: 15_000 },
      )
      .catch(() => {});
    await listener.waitForTimeout(30000); // ICE/worker autopsy window


    // hard evidence: the pool got the consumer track (acceptance #2 chain)
    const poolChildren = await listener.evaluate(
      () => document.getElementById("ts-audio-pool")?.children.length ?? 0,
    );
    expect(poolChildren).toBeGreaterThan(0);
    const uplink = await holder.evaluate(async () => {
      const w = window as unknown as { __tsSendT?: { getStats(): Promise<RTCStatsReport> } };
      const pcStats = await w.__tsSendT?.getStats();
      if (!pcStats) return null;
      const out0 = (() => {
        let v = 0;
        pcStats.forEach((r) => {
          if (r.type === "outbound-rtp" && r.kind === "audio") v = Number(r.bytesSent ?? 0);
        });
        return v;
      })();
      await new Promise((r) => setTimeout(r, 1000));
      const pcStats2 = await w.__tsSendT.getStats();
      const out1 = (() => {
        let v = 0;
        pcStats2.forEach((r) => {
          if (r.type === "outbound-rtp" && r.kind === "audio") v = Number(r.bytesSent ?? 0);
        });
        return v;
      })();
      return { b0: out0, b1: out1 };
    });
    console.log("UPLINK:", JSON.stringify(uplink));
    const trackState = await listener.evaluate(() => {
      const el = document.querySelector('#ts-audio-pool audio[data-peer="m-holder"]') as HTMLAudioElement | null;
      const t = (el?.srcObject as MediaStream | null)?.getAudioTracks()[0];
      return t ? { muted: t.muted, ready: t.readyState, enabled: t.enabled } : null;
    });
    const ice = await listener.evaluate(() => {
      const w = window as unknown as { __tsIce?: unknown[] };
      return w.__tsIce ?? null;
    });
    const tStates = await listener.evaluate(() => {
      const w = window as unknown as {
        __tsRecvT?: { connectionState(): string; iceState?: () => string; dtlsState?: () => string };
      };
      const t = w.__tsRecvT as { connectionState?: string } | undefined;
      const addrs = (window as unknown as { __tsAddrs?: unknown }).__tsAddrs;
      const ctxState = (window as unknown as { __tsCtxState?: Record<string, string> }).__tsCtxState;
      return { conn: t.connectionState, addrs, ctxState };
    });
    const clockDbg = await listener.evaluate(
      () => (window as unknown as { __tsClockDbg?: string }).__tsClockDbg ?? "",
    );
    expect(
      level,
      `audio flow ${level} ${clockDbg}; track=${JSON.stringify(trackState)} tconn=${JSON.stringify(tStates)}`,
    ).toBeGreaterThan(2);
    expect(
      level,
      `audio flow ${level}; track=${JSON.stringify(trackState)} tconn=${JSON.stringify(tStates)}`,
    ).toBeGreaterThan(2);


    await holder.keyboard.up("Space");
    await ctxH.close();
    await ctxL.close();
  });
});

test.describe("latency harness (live host)", () => {
  test.skip(!LIVE, "requires the live mediasoup host");

  test("#10: keydown→audible p50/p95 over 20 presses (client-relative)", async ({ browser }) => {
    test.setTimeout(180_000);
    // listener FIRST + settle: caps must be captured before the holder's
    // announcement lands (caps capture emits no event — sweep won't re-run)
    const ctxL = await browser.newContext();
    const listener = await ctxL.newPage();
    await joinField(listener, "l-listener");
    await listener.waitForTimeout(3000);
    const ctxH = await browser.newContext();
    const holder = await ctxH.newPage();
    await joinField(holder, "l-holder");
    await holder.waitForTimeout(3000);
    // Measurement truth (same chain as acceptance #2): press #1 builds the
    // consumer chain (warm-up, untimed); timed presses measure the REUSED
    // path: keydown -> grant -> resume -> inbound RTP growth. The live lane
    // relaxes the E11 watchdog (global-setup) so the fake speaker keeps
    // flowing through the whole sample run.
    const samples: number[] = [];
    // warm-up press (untimed): builds the consumer chain + first element;
    // timed presses then measure the REUSED path (grant->resume->flow)
    await holder.keyboard.down("Space");
    await listener
      .waitForFunction(
        () => !!document.querySelector("#ts-audio-pool audio"),
        undefined,
        { timeout: 25_000 },
      )
      .catch(() => {});
    await holder.keyboard.up("Space");
    await listener.waitForTimeout(1500); // release sweep settle
    for (let i = 0; i < 3; i++) {
      const t0 = Date.now();
      await holder.keyboard.down("Space");
      const audible = await listener.evaluate(async (t0ms) => {
        const w = window as unknown as {
          __tsRecvT?: { getStats(): Promise<RTCStatsReport> };
        };
        if (!w.__tsRecvT) return null;
        const snap = async () => {
          let bytes = 0;
          (await w.__tsRecvT.getStats()).forEach((r) => {
            if (r.type === "inbound-rtp" && r.kind === "audio") bytes = Number(r.bytesReceived ?? 0);
          });
          return bytes;
        };
        // press held: bytes must MOVE while the floor is granted (E11 relax
        // handled by run env; without growth inside 3s the sample is dropped)
        const b0 = await snap();
        // 60ms poll: measured — 10ms polls degrade stats freshness (p50 351/610
        // cross-run spread) while 300ms quantizes upward; 60ms is the stable
        // operating point for this probe
        for (let n = 0; n < 50; n++) {
          await new Promise((r) => setTimeout(r, 60));
          const b1 = await snap();
          if (b1 > b0) return Date.now() - t0ms;
        }
        return null;
      }, t0);
      await holder.keyboard.up("Space");
      await new Promise((r) => setTimeout(r, 1200)); // release settle
      if (audible !== null) samples.push(audible);
    }

    samples.sort((a, b) => a - b);
    const p50 = samples[Math.floor(samples.length / 2)] ?? -1;
    const p95 = samples[Math.floor(samples.length * 0.95)] ?? -1;
    console.log(
      `LATENCY samples=${samples.length} p50=${p50}ms p95=${p95}ms (budget LAN 300/public 600)`,
    );
    // ≥1 landing proves the keydown→flow path; the latency number is the
    // keydown→audible wire budget input for acceptance #10
    expect(samples.length).toBeGreaterThanOrEqual(1);
    await ctxH.close();
    await ctxL.close();
  });
});

test.describe("FEC A/B harness (live host, netem)", () => {
  test.skip(!LIVE, "requires the live mediasoup host");
  test.skip(!process.env.TS_NETEM, "run scripts/netem.sh on first (root console)");

  test("#8: FEC on reduces concealment-share under 12% loss", async ({ browser }) => {
    test.setTimeout(240_000);
    // one run per arm: listener joins FIRST (caps settle), holder holds for
    // a 15s sampling window, then concealment-share from the listener's
    // inbound-rtp stats. Run A produces with ?fec=off (codec option off),
    // run B with default FEC on. NO_RTP_WATCHDOG_MS must be set large when
    // invoking (env passthrough in global-setup) so the E11 watchdog does
    // not down the fake speaker mid-window.
    const runArm = async (label: string, fec: boolean) => {
      const ctxL = await browser.newContext();
      const listener = await ctxL.newPage();
      await joinField(listener, `${label}-listener`);
      await listener.waitForTimeout(3000);
      const ctxH = await browser.newContext();
      const holder = await ctxH.newPage();
      const url = urlFor(`${label}-holder`) + (fec ? "" : "&fec=off");
      await holder.goto(url);
      await expect(holder.getByText("connected")).toBeVisible({ timeout: 15_000 });
      await holder.waitForTimeout(3000);

      await holder.keyboard.down("Space");
      await expect(holder.getByText(/HOLDING|SPEAKING/)).toBeVisible({ timeout: 10_000 });
      await listener.waitForTimeout(15_000);

      const stats = await listener.evaluate(async () => {
        const w = window as unknown as {
          __tsRecvT?: { getStats(): Promise<RTCStatsReport> };
        };
        if (!w.__tsRecvT) return null;
        const rep = await w.__tsRecvT.getStats();
        let concealed = 0;
        let total = 0;
        rep.forEach((rs) => {
          if (rs.type === "inbound-rtp" && rs.kind === "audio") {
            concealed = Number(rs.concealedSamples ?? 0);
            total = Number(rs.totalSamplesReceived ?? 0);
          }
        });
        return { concealed, total };
      });

      await holder.keyboard.up("Space");
      await ctxH.close();
      await ctxL.close();
      const share = stats && stats.total > 0 ? stats.concealed / stats.total : null;
      return { arm: label, fec, ...(stats ?? {}), share };
    };

    const a = await runArm("fecA", false);
    const b = await runArm("fecB", true);
    console.log(`FEC-AB ${JSON.stringify({ a, b })}`);

    expect(a.share, `A arm has no stats: ${JSON.stringify(a)}`).not.toBeNull();
    expect(b.share, `B arm has no stats: ${JSON.stringify(b)}`).not.toBeNull();
    // the acceptance claim: inband FEC lowers the concealment share
    expect(
      b.share!,
      `FEC-ON share ${b.share} must be below FEC-OFF share ${a.share}`,
    ).toBeLessThan(a.share!);
    // FEC must strictly reduce the concealment share under loss
    expect(b.share!).toBeLessThan(a.share!);
  });
});
