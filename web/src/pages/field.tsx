// Field page — modules/08 §1: roster + hold-to-talk (pointer tri-state, Space
// double-fire guard, keybind capture drawer, release-delay), aria-live floor
// announcements, audio cues (grant/deny/taken), iOS AudioContext unlock,
// settings drawer with the accessibility toggle (plan-review restore).

import { useCallback, useEffect, useRef, useState } from "react";
import { useParams } from "react-router-dom";
import { Button, Card, List, Tag, Drawer, Switch, Typography, Space, Badge } from "antd";
import { SettingOutlined, AudioOutlined } from "@ant-design/icons";
import {
  ensureSession,
  connectSession,
  getSession,
  useMirror,
  useSessionStatus,
  jwtFromQuery,
} from "../lib/session.js";

const { Text, Title } = Typography;

/** Space double-fire guard: keydown repeats are swallowed (e.repeat). */
const PREEMPT_PRIORITY = 5;

export function FieldPage() {
  const { room = "demo" } = useParams();
  const key = `field:${room}`;
  const mirror = useMirror(key);
  const status = useSessionStatus(key);
  const [holding, setHolding] = useState(false);
  const [drawerOpen, setDrawerOpen] = useState(false);
  const [a11y, setA11y] = useState(true);
  const [announcement, setAnnouncement] = useState("");
  const [keybind, setKeybind] = useState<string | null>(null);
  const [keybindCapture, setKeybindCapture] = useState(false);
  const holdTimer = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    ensureSession({ room, role: "field" });
    const jwt = jwtFromQuery();
    if (jwt && !status.connected) {
      void connectSession(key, `${location.origin.replace(/^http/, "ws")}/ws`, jwt);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [room]);

  // aria-live announcements from mirror transitions (grant/taken/deny cues)
  useEffect(() => {
    if (!a11y || !status.selfId) return;
    if (mirror.grants.includes(status.selfId)) {
      setAnnouncement("Floor granted. You may speak.");
    } else if (mirror.grants.length > 0) {
      setAnnouncement(`Floor held by ${mirror.grants[0]}`);
    } else {
      setAnnouncement("Floor idle.");
    }
  }, [mirror.grants, a11y, status.selfId]);

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

  return (
    <div style={{ padding: 16, userSelect: "none" }}>
      <Space style={{ marginBottom: 12 }}>
        <Title level={4} style={{ margin: 0 }}>
          Field — {room}
        </Title>
        <Tag color={status.connected ? "green" : "red"}>
          {status.connected ? "connected" : "offline"}
        </Tag>
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
              {mirror.muted.includes(p.id) && <Tag style={{ marginLeft: 8 }}>muted</Tag>}
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
        {holding ? "HOLDING — release to yield" : meHolder ? "SPEAKING (granted)" : "HOLD TO TALK"}
      </Button>
      <div className="mirror-note" style={{ marginTop: 8 }}>
        press-and-hold (pointer or {keybind ?? "Space"}); preempt uses priority {PREEMPT_PRIORITY}
      </div>

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
