// Participant page — /room/:room: the conference shell. Join link in, mic
// toggle, live speaker grid. Rides the field role's wire session (no new
// Role variant); D13 transmission gating keeps the mic paused until granted,
// so speaking state is always server-driven — the page never guesses.

import { useCallback, useEffect, useState } from "react";
import { useParams } from "react-router-dom";
import { Button, Card, Tag, Typography, Space } from "antd";
import { AudioMutedOutlined, AudioOutlined } from "@ant-design/icons";
import {
  ensureSession,
  connectSession,
  getSession,
  useMirror,
  useSessionStatus,
  jwtFromQuery,
} from "../lib/session.js";
import {
  MUTED_BY_DISPATCHER,
  REMOVED_BY_DISPATCHER,
  denialCopy,
} from "../lib/floorCopy.js";

const { Title, Text } = Typography;

export function ParticipantPage() {
  const { room = "demo" } = useParams();
  // shared with the field page BY DESIGN: same role session, thinner shell
  const key = `field:${room}`;
  const mirror = useMirror(key);
  const status = useSessionStatus(key);
  const [micOn, setMicOn] = useState(false);

  useEffect(() => {
    const session = ensureSession({ room, role: "field" });
    const jwt = jwtFromQuery();
    if (jwt && !status.connected) {
      void connectSession(key, `${location.origin.replace(/^http/, "ws")}/ws`, jwt);
    }
    // remote tracks autoplay into the pool (media slice; the field page owns
    // the analyser diagnostics harness, this shell keeps it minimal)
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
    });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [room]);

  const toggleMic = useCallback(() => {
    const s = getSession(key);
    if (!s) return;
    if (micOn) {
      s.client.releaseFloor();
      setMicOn(false);
    } else {
      s.client.requestFloor(0, false);
      setMicOn(true);
    }
  }, [key, micOn]);

  // A refusal must not leave the toggle stuck on "REQUESTING FLOOR…" (the optimistic
  // micOn has no server answer behind it once the floor says no). Keyed on the event
  // stream: two refusals carry the same reason string, which would not re-fire this.
  useEffect(() => {
    if (status.events[0]?.kind === "denied") setMicOn(false);
  }, [status.events]);

  const speaking = status.selfId ? mirror.grants.includes(status.selfId) : false;
  // S7.5 deny != mute: standing ban (roster) and one-off refusal are different lines.
  // R7-b: a removal outranks both — it is terminal for this session.
  const meMuted = !!status.selfId && mirror.muted.includes(status.selfId);
  const removed = status.removedReason;
  const refusal =
    !removed && !meMuted && !speaking && status.lastDenial
      ? denialCopy(status.lastDenial, false)
      : null;
  const speakers = mirror.peers.filter((p) => mirror.grants.includes(p.id));

  return (
    <div style={{ padding: 16, userSelect: "none" }}>
      <Space style={{ marginBottom: 12 }}>
        <Title level={4} style={{ margin: 0 }}>Room — {room}</Title>
        <Tag color={status.connected ? "green" : "red"}>
          {status.connected ? "connected" : "offline"}
        </Tag>
        <Tag>mode: {mirror.mode}</Tag>
      </Space>

      <Card size="small" title={`Speakers (${speakers.length})`} style={{ marginBottom: 12 }}>
        {speakers.length === 0 && <Text type="secondary">nobody is talking</Text>}
        <Space wrap>
          {speakers.map((p) => (
            <Tag key={p.id} color="red">
              {p.id}
              {p.id === status.selfId ? " (me)" : ""}
            </Tag>
          ))}
        </Space>
      </Card>

      <Button
        size="large"
        block
        type={micOn ? "primary" : "default"}
        danger={speaking}
        icon={micOn ? <AudioMutedOutlined /> : <AudioOutlined />}
        style={{ height: 88, fontSize: 18 }}
        disabled={!status.connected}
        onClick={toggleMic}
      >
        {/* a removal is terminal and outranks the offline wording: "waiting for
            connection" would promise a retry that will never come */}
        {removed
          ? "REMOVED BY DISPATCHER"
          : !status.connected
            ? "WAITING FOR CONNECTION"
            : meMuted
              ? "MUTED BY DISPATCHER"
              : speaking
                ? "ON AIR — click to mute"
                : micOn
                  ? "REQUESTING FLOOR…"
                  : "JOIN THE CONVERSATION"}
      </Button>

      <div className="mirror-note" style={{ marginTop: 8 }}>
        gen {mirror.generation} — you speak only while granted ({mirror.mode} mode;
        dispatcher can open the floor to everyone)
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
      <div id="ts-audio-pool" style={{ display: "none" }} />
    </div>
  );
}
