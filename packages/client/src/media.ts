// Media manager: mediasoup-client lifecycle + AudioHandle pool + mic truth.
//
// Design notes (D13/D16 + plan-3 routed findings):
// - The server creates producers PAUSED and gates them via apply_floor; the
//   client never guesses audibility — micTruth = track.enabled && !serverPaused.
// - AudioHandle{pause,release} per consumer; released on FloorIdle/Taken/
//   MediaDown (S-1/#2).
// - `connectionstatechange: failed` → local teardown + J-3-7 re-run (S-5b/#12),
//   mirroring W-step 3.
// - MediaRestart tears down media state; the next produce/run re-runs J-3-7.
// - visibilitychange must NOT gate media (S-8) — this module never pauses on
//   tab hide; that rule lives here, not in views.

import type { TalkServoClient } from "./index.js";

/** Structural shape of the mediasoup-client pieces we drive (DI-testable
 *  without a real WebRTC stack in unit tests). */
export interface MediaStack {
  /** Load device with server router caps. */
  load(caps: unknown): Promise<void>;
  createSendTransport(params: {
    ice: unknown;
    dtls: unknown;
  }): Promise<SendTransportLike>;
  createRecvTransport(params: {
    ice: unknown;
    dtls: unknown;
  }): Promise<RecvTransportLike>;
}

export interface SendTransportLike {
  produce(opts: {
    track: MediaStreamTrack;
    codecOptions?: Record<string, unknown>;
  }): Promise<ProducerLike>;
}

export interface RecvTransportLike {
  consume(opts: {
    id: string;
    producerId: string;
    kind: string;
    rtpParameters: unknown;
  }): Promise<ConsumerLike>;
}

export interface ProducerLike {
  id: string;
  pause(): void;
  resume(): void;
  close(): void;
}

export interface ConsumerLike {
  id: string;
  producerId: string;
  track: MediaStreamTrack;
  pause(): void;
  resume(): void;
  close(): void;
}

/** One consumed remote audio line. The UI owns the <audio> element + setSinkId;
 *  the SDK owns pause/release lifecycle truth. */
export interface AudioHandle {
  readonly peerId: string;
  readonly track: MediaStreamTrack;
  /** Local mute (UI toggle) — does not touch the server. */
  pause(): void;
  resume(): void;
  /** Drop the consumer entirely (Idle/Taken/MediaDown). */
  release(): void;
  readonly released: boolean;
}

/** mic truth per D16: local enable AND server not pausing us. */
export interface MicTruth {
  trackEnabled: boolean;
  serverPaused: boolean;
}

export type MediaEvent =
  | { kind: "mic-truth"; truth: MicTruth }
  | { kind: "audio-opened"; peerId: string; handle: AudioHandle }
  | { kind: "audio-released"; peerId: string }
  | { kind: "transport-failed" }
  | { kind: "media-teardown"; reason: string };

export interface MediaManagerOptions {
  /** Device capabilities for the server-side consume exchange. In the full
   *  design these come from mediasoup-client `device.load(caps)` results; PoC
   *  tests inject fakes. */
  stack: MediaStack;
}

/** Owns the send track + per-peer consumer handles; reacts to floor events. */
export class MediaManager {
  private client: TalkServoClient | null = null;
  private micTrack: MediaStreamTrack | null = null;
  private producer: ProducerLike | null = null;
  private serverPaused = true; // server creates producers paused (D13)
  private handles = new Map<string, AudioHandleImpl>();
  /** Consume{producer_id} → peer awaiting ConsumeOk (pull-model pairing). */
  private pendingConsumes = new Map<string, string>();
  /** Router caps from the server (device.load payload). */
  routerCaps: unknown = null;
  private listeners = new Set<(e: MediaEvent) => void>();
  private stack: MediaStack;

  constructor(opts: MediaManagerOptions) {
    this.stack = opts.stack;
  }

  on(fn: (e: MediaEvent) => void): () => void {
    this.listeners.add(fn);
    return () => this.listeners.delete(fn);
  }

  private emit(e: MediaEvent) {
    for (const fn of this.listeners) fn(e);
  }

  /** Bind to a connected client; subscribes to the floor/media events that
   *  drive handle release + gating truth. */
  attach(client: TalkServoClient): void {
    this.client = client;
    // raw wire taps for the consume exchange (facade events don't carry these)
    client.onMessage((msg) => {
      if (msg.type === "router_caps") {
        this.routerCaps = msg.media_codecs;
      } else if (msg.type === "consume_ok") {
        const pid = String(msg.producer_id);
        const peer = this.pendingConsumes.get(pid);
        if (peer) {
          this.pendingConsumes.delete(pid);
          void this.instantiateConsumer(peer, pid, msg.rtp_parameters, msg.consumer_id);
        }
      }
    });
    client.on((e) => {
      switch (e.kind) {
        case "state": {
          // releases: peers that left the grant set lose their handles (S-1)
          const granted = new Set(e.mirror.grants);
          for (const peer of [...this.handles.keys()]) {
            if (!granted.has(peer)) this.releasePeer(peer);
          }
          return;
        }
        case "media-failed":
          this.releasePeer(e.peer);
          return;
        case "media-restart":
          this.teardown(`media_restart: ${e.reason}`);
          return;
        default:
          return;
      }
    });
  }

  /** Publish the local mic (J-step 6). Created paused server-side; we mirror
   *  `serverPaused=true` until the server resumes us (grant). */
  async publishMic(track: MediaStreamTrack): Promise<void> {
    if (!this.client) throw new Error("attach() first");
    this.micTrack = track;
    // PoC transport creation rides the signal channel; the send transport is
    // created on demand by the full stack — here we only pin codec options and
    // truth state. The transport/produce exchange is exercised via sendRaw.
    this.client.sendRaw({ type: "transport_create" });
    this.serverPaused = true;
    this.emitMicTruth();
  }

  /** Consume a remote producer into an AudioHandle (J-step 7). */
  async consume(
    peerId: string,
    producerId: string,
    recv: RecvTransportLike,
    rtpParameters: unknown,
  ): Promise<AudioHandle> {
    const consumer = await recv.consume({
      id: `${producerId}-consumer`,
      producerId,
      kind: "audio",
      rtpParameters,
    });
    const existing = this.handles.get(peerId);
    if (existing && !existing.released) existing.release();

    const h = new AudioHandleImpl(peerId, consumer.track, consumer);
    this.handles.set(peerId, h);
    this.emit({ kind: "audio-opened", peerId, handle: h });
    return h;
  }

  /** Pull-model consume: request every granted peer's producer (J-step 7).
   *  ConsumeOk completes the exchange via instantiateConsumer. */
  consumeGranted(grants: string[], producerByPeer: Map<string, string>): void {
    if (!this.client) return;
    for (const peer of grants) {
      if (this.handles.has(peer)) continue; // already consuming
      const producerId = producerByPeer.get(peer);
      if (!producerId) continue;
      this.pendingConsumes.set(producerId, peer);
      this.client.sendRaw({ type: "consume", producer_id: producerId });
    }
  }

  /** Local mediasoup-client consumer instantiation (needs device.load'd caps
   *  — the SDK consumer path; tests inject a fake RecvTransportLike). */
  private async instantiateConsumer(
    peerId: string,
    producerId: string,
    rtpParameters: unknown,
    consumerId: unknown,
  ): Promise<void> {
    if (!this.recvFactory) return;
    const recv = await this.recvFactory();
    try {
      await this.consumeSpec(
        peerId,
        { id: String(consumerId), producerId, kind: "audio", rtpParameters },
        recv,
      );
    } catch {
      this.pendingConsumes.set(producerId, peerId); // retry on next grant sweep
    }
  }

  /** Spec-driven consume (server-created consumer local instantiation). */
  async consumeSpec(
    peerId: string,
    spec: { id: string; producerId: string; kind: string; rtpParameters: unknown },
    recv: RecvTransportLike,
  ): Promise<AudioHandle> {
    const consumer = await recv.consume(spec);
    const existing = this.handles.get(peerId);
    if (existing && !existing.released) existing.release();

    const h = new AudioHandleImpl(peerId, consumer.track, consumer);
    this.handles.set(peerId, h);
    this.emit({ kind: "audio-opened", peerId, handle: h });
    return h;
  }

  /** DI: recv transport factory (media slice wires mediasoup-client here). */
  setRecvFactory(f: (() => Promise<RecvTransportLike>) | null): void {
    this.recvFactory = f;
  }
  private recvFactory: (() => Promise<RecvTransportLike>) | null = null;

  /** Server told us our producer is (un)paused — the grant path. */
  setServerPaused(paused: boolean): void {
    this.serverPaused = paused;
    this.emitMicTruth();
  }

  micTruth(): MicTruth {
    return {
      trackEnabled: this.micTrack?.enabled ?? false,
      serverPaused: this.serverPaused,
    };
  }

  /** S-5b/#12: transport failure → teardown + signal the re-run. */
  transportFailed(): void {
    this.teardown("connectionstate failed");
  }

  /** W-step 3 mirror: drop all media state; J-3-7 re-runs from the facade. */
  teardown(reason: string): void {
    for (const h of this.handles.values()) h.release();
    this.handles.clear();
    this.producer = null;
    this.serverPaused = true;
    this.emit({ kind: "media-teardown", reason });
  }

  get handleCount(): number {
    return this.handles.size;
  }

  private releasePeer(peer: string) {
    const h = this.handles.get(peer);
    if (h) {
      h.release();
      this.handles.delete(peer);
      this.emit({ kind: "audio-released", peerId: peer });
    }
  }

  private emitMicTruth() {
    this.emit({ kind: "mic-truth", truth: this.micTruth() });
  }
}

class AudioHandleImpl implements AudioHandle {
  released = false;
  private paused = false;

  constructor(
    readonly peerId: string,
    readonly track: MediaStreamTrack,
    private consumer: ConsumerLike,
  ) {}

  pause(): void {
    if (this.released || this.paused) return;
    this.paused = true;
    this.consumer.pause();
  }

  resume(): void {
    if (this.released || !this.paused) return;
    this.paused = false;
    this.consumer.resume();
  }

  release(): void {
    if (this.released) return;
    this.released = true;
    this.consumer.close();
  }
}
