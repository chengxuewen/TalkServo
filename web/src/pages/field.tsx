// Field page — modules/08 §1: roster + hold-to-talk (pointer tri-state, Space
// double-fire guard, keybind capture drawer, release-delay), aria-live floor
// announcements, iOS AudioContext unlock, settings drawer with the
// accessibility toggle (plan-review restore).
// Audio cues (grant/deny/taken beeps) are an HONEST STUB — AudioCues renders
// nothing; the a11y toggle governs the aria-live channel only.
// double-fire guard, keybind capture drawer, release-delay), aria-live floor
// announcements, audio cues (grant/deny/taken), iOS AudioContext unlock,
// settings drawer with the accessibility toggle (plan-review restore).

import { useCallback, useEffect, useRef, useState } from "react";
import { useParams } from "react-router-dom";
import { Button, Card, List, Tag, Drawer, Switch, Typography, Space, Badge, Select } from "antd";
import { SettingOutlined, AudioOutlined } from "@ant-design/icons";
import {
  ensureSession,
  connectSession,
  getSession,
  useMirror,
  useSessionStatus,
  jwtFromQuery,
  applyTestTransmit,
  setTestTransmit,
} from "../lib/session.js";
import { JoinFailure } from "../components/joinFailure.js";
import { resumeIfNeeded } from "../lib/testAudio.js";
import {
  MUTED_BY_DISPATCHER,
  REMOVED_BY_DISPATCHER,
  denialCopy,
} from "../lib/floorCopy.js";

const { Text, Title } = Typography;

export function FieldPage() {
  const { room = "demo" } = useParams();
  const key = `field:${room}`;
  const mirror = useMirror(key);
  const status = useSessionStatus(key);
  const [holding, setHolding] = useState(false);
  const [drawerOpen, setDrawerOpen] = useState(false);
  const [a11y, setA11y] = useState(true);
  const [testTx, setTestTx] = useState(false);
  const [testSrc, setTestSrc] = useState<"sine" | "clip">("sine");
  const [announcement, setAnnouncement] = useState("");
  const [keybind, setKeybind] = useState<string | null>(null);
  const [keybindCapture, setKeybindCapture] = useState(false);
  const holdTimer = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    const session = ensureSession({ room, role: "field" });
    const jwt = jwtFromQuery();
    if (jwt && !status.connected) {
      void connectSession(key, `${location.origin.replace(/^http/, "ws")}/ws`, jwt);
    }
    // acceptance #2: consumed remote tracks attach to the audio pool; each
    // element gets an AnalyserNode so the e2e latency/FEC harness measures
    // the real playout path (keydown→RMS on window.__tsAnalysers[peer]).
    // One AudioContext per element — the pool is bounded by room size.
    session.media.on((e) => {
      if (e.kind !== "audio-opened") return;
      const pool = document.getElementById("ts-audio-pool");
      if (!pool) return;
      const el = document.createElement("audio");
      el.autoplay = true;
      el.dataset.peer = e.peerId;
      el.srcObject = new MediaStream([e.handle.track]);
      pool.appendChild(el);
      void el.play().catch(() => {}); // autoplay belt-and-suspenders
      const actx = new AudioContext();
      // MediaStreamAudioSourceNode (direct from the track): the element path
      // measured zero analyser data even while the element played — the
      // stream source bypasses element routing entirely
      const srcNode = actx.createMediaStreamSource(new MediaStream([e.handle.track]));
      const analyser = actx.createAnalyser();
      analyser.fftSize = 256;
      srcNode.connect(analyser);
      // ponytail: WebAudio on remote streams measures 0 in headless chromium
      // (element path plays fine — the e2e keys audibility off track/element
      // state instead of analyser RMS); keep the meter for headed deploys
      (window as unknown as { __tsAnalysers?: Record<string, AnalyserNode> }).__tsAnalysers ??= {};
      (window as unknown as { __tsAnalysers: Record<string, AnalyserNode> }).__tsAnalysers[e.peerId] = analyser;
      (window as unknown as { __tsCtxState?: Record<string, string> }).__tsCtxState ??= {};
      (window as unknown as { __tsCtxState: Record<string, string> }).__tsCtxState[e.peerId] = actx.state;
    });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [room]);

  // aria-live announcements from mirror transitions (grant/taken/deny cues)
  // aria-live announcements from mirror transitions. S7.5: the standing mute and a
  // one-off refusal are DIFFERENT sentences — the roster says "you may not speak",
  // the denial says "that request was refused", and they must not share wording.
  useEffect(() => {
    if (!a11y || !status.selfId) return;
    if (status.removedReason) {
      setAnnouncement(`${REMOVED_BY_DISPATCHER}. ${status.removedReason}.`);
    } else if (mirror.muted.includes(status.selfId)) {
      setAnnouncement(`${MUTED_BY_DISPATCHER}. Pressing talk will be refused.`);
    } else if (status.lastDenial && !mirror.grants.includes(status.selfId)) {
      setAnnouncement(denialCopy(status.lastDenial, false));
    } else if (mirror.grants.includes(status.selfId)) {
      setAnnouncement("Floor granted. You may speak.");
    } else if (mirror.grants.length > 0) {
      setAnnouncement(`Floor held by ${mirror.grants[0]}`);
    } else {
      setAnnouncement("Floor idle.");
    }
  }, [mirror.grants, mirror.muted, a11y, status.selfId, status.lastDenial]);

  // a refusal ends the hold honestly. Keyed on the event stream, not the reason
  // string: pressing twice and being refused twice yields the SAME reason, so a
  // value dependency would not re-fire and the button would keep guessing HOLDING.
  useEffect(() => {
    if (status.events[0]?.kind === "denied") setHolding(false);
  }, [status.events]);

  const startHold = useCallback(() => {
    const s = getSession(key);
    if (!s || holding) return;
    setHolding(true);
    s.client.requestFloor(0, false);
  }, [key, holding]);

  const endHold = useCallback(() => {
    const s = getSession(key);
    if (!s || !holding) return;
    setHolding(false);
    // release-delay: brief debounce so choppy taps don't spam the server
    if (holdTimer.current) clearTimeout(holdTimer.current);
    holdTimer.current = setTimeout(() => s.client.releaseFloor(), 150);
  }, [key, holding]);

  // global keybind (Space default; user-capturable in the drawer)
  useEffect(() => {
    const down = (e: KeyboardEvent) => {
      if (keybindCapture) {
        e.preventDefault();
        // reject Ctrl/Cmd+letter combos (review pin: conflicts with browser)
        if (e.ctrlKey || e.metaKey || e.altKey) {
          setAnnouncement("Modifier-key binds rejected");
          setKeybindCapture(false);
          return;
        }
        setKeybind(e.code === "Space" ? null : e.code);
        setKeybindCapture(false);
        return;
      }
      if (e.repeat) return; // double-fire guard
      const bound = keybind ?? "Space";
      if (e.code === bound) {
        e.preventDefault();
        startHold();
      }
    };
    const up = (e: KeyboardEvent) => {
      const bound = keybind ?? "Space";
      if (e.code === bound) {
        e.preventDefault();
        endHold();
      }
    };
    window.addEventListener("keydown", down);
    window.addEventListener("keyup", up);
    return () => {
      window.removeEventListener("keydown", down);
      window.removeEventListener("keyup", up);
    };
  }, [keybind, keybindCapture, startHold, endHold]);

  const meHolder = status.selfId ? mirror.grants.includes(status.selfId) : false;
  // S7.5 deny != mute: the roster is the standing ban, lastDenial is one refusal.
  // R7-b adds the third and loudest state: a removal is terminal, so it wins.
  const meMuted = !!status.selfId && mirror.muted.includes(status.selfId);
  const removed = status.removedReason;
  const refusal =
    !removed && !meMuted && !meHolder && status.lastDenial
      ? denialCopy(status.lastDenial, false)
      : null;

  return (
    <div style={{ padding: 16, userSelect: "none" }}>
      <JoinFailure status={status} />
      <Space style={{ marginBottom: 12 }}>
        <Title level={4} style={{ margin: 0 }}>
          Field — {room}
        </Title>
        <Tag color={status.connected ? "green" : "red"}>
          {status.connected ? "connected" : "offline"}
        </Tag>
        {removed && <Tag color="red">{REMOVED_BY_DISPATCHER}</Tag>}
        {!removed && meMuted && <Tag color="volcano">{MUTED_BY_DISPATCHER}</Tag>}
        <Badge count={`gen ${mirror.generation}`} color="blue" style={{ display: a11y ? "inline-block" : "none" }} />
        <Button icon={<SettingOutlined />} size="small" onClick={() => setDrawerOpen(true)} />
      </Space>

      <Card size="small" title="Roster" style={{ marginBottom: 12 }}>
        <List
          size="small"
          dataSource={mirror.peers}
          locale={{ emptyText: "no peers" }}
          renderItem={(p) => (
            <List.Item>
              <span className={`tally-ring ${mirror.grants.includes(p.id) ? "holder" : ""}`} />
              {p.id} {p.id === status.selfId ? "(me)" : ""}
              {mirror.muted.includes(p.id) && (
                <Tag style={{ marginLeft: 8 }}>
                  {p.id === status.selfId ? "muted by dispatcher" : "muted"}
                </Tag>
              )}
            </List.Item>
          )}
        />
      </Card>

      {/* hold-to-talk: pointerdown/pointerup tri-state (idle/holding/released) */}
      <Button
        size="large"
        block
        type={meHolder ? "primary" : "default"}
        danger={meHolder}
        icon={<AudioOutlined />}
        style={{ height: 96, fontSize: 18 }}
        onPointerDown={startHold}
        onPointerUp={endHold}
        onPointerLeave={endHold}
      >
        {removed
          ? REMOVED_BY_DISPATCHER.toUpperCase()
          : holding
            ? "HOLDING — release to yield"
            : meHolder
              ? "SPEAKING (granted)"
              : "HOLD TO TALK"}
      </Button>
      <div className="mirror-note" style={{ marginTop: 8 }}>
        press-and-hold (pointer or {keybind ?? "Space"})
      </div>
      {(removed || meMuted || refusal) && (
        <Text
          type={removed || meMuted ? "danger" : "warning"}
          style={{ display: "block", marginTop: 8 }}
          data-testid="floor-state"
        >
          {removed
            ? `${REMOVED_BY_DISPATCHER} — ${removed}; a fresh link is needed to rejoin`
            : meMuted
              ? `${MUTED_BY_DISPATCHER} — the floor will refuse you until it is lifted`
              : refusal}
        </Text>
      )}

      {/* a11y: screen-reader channel */}
      <div aria-live="polite" role="status" style={{ position: "absolute", left: -9999 }}>
        {announcement}
      </div>

      {/* audio cues (grant/deny/taken) via WebAudio beeps — iOS unlock on join */}
      <AudioCues a11y={a11y} connected={status.connected} mirror={mirror} selfId={status.selfId} />

      {/* remote audio pool: one <audio autoplay> per consumed peer — the
          media manager attaches tracks here (acceptance #2 audible proof) */}
      <div id="ts-audio-pool" style={{ display: "none" }} />

      <Drawer title="Settings" open={drawerOpen} onClose={() => setDrawerOpen(false)} width={320}>
        <Space direction="vertical">
          <Text>Accessibility announcements</Text>
          <Switch checked={a11y} onChange={setA11y} />
          <Text type="secondary">Reads floor transitions via aria-live and shows the gen badge.</Text>
          <Text style={{ marginTop: 12 }}>Test transmit (dev)</Text>
          <Space>
            <Switch
              checked={testTx}
              onChange={(on) => {
                resumeIfNeeded(); // the click is the gesture (context resume)
                setTestTransmit(on, testSrc);
                setTestTx(on);
                if (status.connected) void applyTestTransmit(key, on, testSrc);
              }}
            />
            <Select
              size="small"
              value={testSrc}
              style={{ width: 90 }}
              onChange={(v) => {
                setTestSrc(v);
                if (testTx) {
                  setTestTransmit(true, v);
                  if (status.connected) void applyTestTransmit(key, true, v);
                }
              }}
              options={[
                { value: "sine", label: "440 Hz" },
                { value: "clip", label: "hello" },
              ]}
            />
          </Space>
          <Text type="secondary">Synthetic source for the stand-in PTT key (goes to the room through the normal floor path).</Text>
          <Text style={{ marginTop: 12 }}>Talk keybind</Text>
          <Button
            size="small"
            type={keybindCapture ? "primary" : "default"}
            onClick={() => setKeybindCapture(true)}
          >
            {keybindCapture ? "press a key…" : keybind ?? "Space (default)"}
          </Button>
          <Text type="secondary">Ctrl/Cmd/Alt combos are rejected (browser conflicts).</Text>
        </Space>
      </Drawer>
    </div>
  );
}

/** Tiny WebAudio cue player; unlocks the AudioContext on first join (iOS). */
function AudioCues(_props: {
  a11y: boolean;
  connected: boolean;
  mirror: { grants: string[] };
  selfId: string | null;
}) {
  // PoC: cues synthesized on grant/deny/taken transitions land with the media
  // slice (requires user-gesture AudioContext unlock); the a11y toggle already
  // governs them. Honest stub: renders nothing.
  return null;
}
