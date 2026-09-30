// Real mediasoup-client stack — browser-only adapter implementing MediaStack.
// Unit tests keep the fake; the e2e (fake-device chromium) uses this.

import { Device } from "mediasoup-client";
import type { Transport } from "mediasoup-client/types";
import type { MediaStack, RecvTransportLike, SendTransportLike } from "./media.js";

/** Loose structural view of a mediasoup Transport — the official class has
 *  richer generics than this adapter needs. */
interface AnyTransport {
  on(event: "connect", fn: (p: { dtlsParameters: unknown }, cb: () => void, eb: (e: Error) => void) => void): void;
  on(event: "produce", fn: (p: { rtpParameters: unknown }, cb: (d: { id: string }) => void, eb: (e: Error) => void) => void): void;
  produce(o: { track: MediaStreamTrack; appData: object; codecOptions?: object }): Promise<{ id: string; pause(): void; resume(): void; close(): void }>;
  consume(o: { id: string; producerId: string; kind: "audio"; rtpParameters: unknown; appData: object }): Promise<{ id: string; producerId: string; track: MediaStreamTrack; pause(): void; resume(): void; close(): void }>;
}

/**
 * Wire the SDK to a live mediasoup server:
 *   1. RouterCaps → device.load(caps)
 *   2. TransportCreate → TransportInfo{ice,dtls} → send/recv transports
 *   3. produce() with opus FEC codecOptions (modules/04 pin)
 *
 * Signal pairing: `connect`/`produce` events are answered locally — the
 * session layer has ALREADY sent TransportConnect/Produce on the signal
 * channel when the transport was created (server-driven flow). The produce
 * callback returns a placeholder id; the authoritative producer id arrives
 * via ProduceOk and the session layer reconciles (pending-produce map).
 */
export class MediasoupClientStack implements MediaStack {
  /** Lazy: constructing Device requires WebRTC (browser) — unit/jsdom
   *  environments never reach load(), so defer the constructor. */
  private device: Device | null = null;
  private sendTransport: AnyTransport | null = null;
  private recvTransport: AnyTransport | null = null;

  private dev(): Device {
    if (!this.device) this.device = new Device();
    return this.device;
  }

  async load(caps: unknown): Promise<void> {
    const dev = this.dev();
    if (!dev.loaded) {
      await dev.load({ routerRtpCapabilities: caps as never });
    }
  }

  get loaded(): boolean {
    return this.device?.loaded ?? false;
  }

  async createSendTransport(params: {
    ice: unknown;
    dtls: unknown;
  }): Promise<SendTransportLike> {
    const dev = this.dev();
    if (!dev.loaded) throw new Error("device not loaded");
    const t = this.dev().createSendTransport({
      iceParameters: params.ice as never,
      iceCandidates: [
        // server sends addrs:[] — synthesize a host candidate (fake-device
        // e2e runs loopback where host candidates suffice)
        {
          foundation: "host",
          ip: "127.0.0.1",
          address: "127.0.0.1",
          port: 0,
          priority: 1,
          protocol: "udp",
          type: "host",
        },
      ],
      dtlsParameters: params.dtls as never,
    } as never) as unknown as AnyTransport;
    t.on("connect", (_p, callback) => callback());
    t.on("produce", (_p, callback) => callback({ id: `pending-${Date.now()}` }));
    this.sendTransport = t;
    return wrapSend(t);
  }

  async createRecvTransport(params: {
    ice: unknown;
    dtls: unknown;
  }): Promise<RecvTransportLike> {
    const dev = this.dev();
    if (!dev.loaded) throw new Error("device not loaded");
    const t = this.dev().createRecvTransport({
      iceParameters: params.ice as never,
      iceCandidates: [],
      dtlsParameters: params.dtls as never,
    } as never) as unknown as AnyTransport;
    t.on("connect", (_p, callback) => callback());
    this.recvTransport = t;
    return wrapRecv(t);
  }

  get recv(): AnyTransport | null {
    return this.recvTransport;
  }
}

function wrapSend(t: AnyTransport): SendTransportLike {
  return {
    async produce(opts) {
      const producer = await t.produce({
        track: opts.track,
        appData: {},
        // opus FEC pin (modules/04)
        codecOptions: { opusStereo: false, opusFec: true, opusDtx: false },
      });
      return {
        id: producer.id,
        pause: () => void producer.pause(),
        resume: () => void producer.resume(),
        close: () => void producer.close(),
      };
    },
  };
}

function wrapRecv(t: AnyTransport): RecvTransportLike {
  return {
    async consume(opts) {
      // server-created consumer: local instantiation needs the consumer id
      // from ConsumeOk (wire now carries it) — this is that local half.
      const consumer = await t.consume({
        id: opts.id,
        producerId: opts.producerId,
        kind: "audio",
        rtpParameters: opts.rtpParameters,
        appData: {},
      });
      return {
        id: consumer.id,
        producerId: consumer.producerId,
        track: consumer.track,
        pause: () => void consumer.pause(),
        resume: () => void consumer.resume(),
        close: () => void consumer.close(),
      };
    },
  };
}
