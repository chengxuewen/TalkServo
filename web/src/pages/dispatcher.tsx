// Dispatcher console — modules/08 §1: target grid + tally rings, event log
// (generation visible HERE only), per-line mute/gain, grant-queue strip,
// authorized actions (ModeChange / MuteSet / preempt).

import { useEffect } from "react";
import { useParams } from "react-router-dom";
import { Card, Col, Row, Tag, Button, Space, List, Typography, Slider, Switch, Badge } from "antd";
import {
  AudioMutedOutlined,
  AudioOutlined,
  CrownOutlined,
  StopOutlined,
} from "@ant-design/icons";
import {
  ensureSession,
  connectSession,
  getSession,
  useMirror,
  useSessionStatus,
  jwtFromQuery,
} from "../lib/session.js";

const { Text, Title } = Typography;

export function DispatcherPage() {
  const { room = "demo" } = useParams();
  const key = `dispatch:${room}`;
  const mirror = useMirror(key);
  const status = useSessionStatus(key);

  useEffect(() => {
    ensureSession({ room, role: "dispatch" });
    const jwt = jwtFromQuery();
    if (jwt && !status.connected) {
      void connectSession(key, `${location.origin.replace(/^http/, "ws")}/ws`, jwt);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [room]);

  const isHolder = (peer: string) => mirror.grants.includes(peer);
  const meHolder = status.selfId ? isHolder(status.selfId) : false;

  return (
    <div style={{ padding: 16 }}>
      <Space style={{ marginBottom: 12 }}>
        <Title level={4} style={{ margin: 0 }}>
          Dispatch — {room}
        </Title>
        <Tag color={status.connected ? "green" : "red"}>
          {status.connected ? "connected" : "offline"}
        </Tag>
        <Tag>mode: {mirror.mode}</Tag>
        <Badge count={`gen ${mirror.generation}`} color="blue" />
      </Space>

      {/* authorized actions */}
      <Space style={{ marginBottom: 16 }}>
        <Button
          size="small"
          onClick={() => mirror.mode !== "exclusive" && status.selfId && sessionAction(key, { type: "mode_change", mode: "exclusive" })}
        >
          Exclusive
        </Button>
        <Button
          size="small"
          onClick={() => mirror.mode !== "open" && sessionAction(key, { type: "mode_change", mode: "open" })}
        >
          Open
        </Button>
        <Button
          size="small"
          onClick={() => mirror.mode !== "hybrid" && sessionAction(key, { type: "mode_change", mode: "hybrid" })}
        >
          Hybrid
        </Button>
        <Button
          size="small"
          danger
          icon={<StopOutlined />}
          onClick={() => status.selfId && sessionAction(key, { type: "floor_release" })}
        >
          Force release
        </Button>
      </Space>

      <Row gutter={16}>
        {/* target grid */}
        <Col span={14}>
          <Card title="Targets" size="small">
            {mirror.peers.length === 0 && <Text type="secondary">no peers</Text>}
            {mirror.peers.map((p) => (
              <Card
                key={p.id}
                size="small"
                style={{ marginBottom: 8 }}
                title={
                  <span>
                    <span className={`tally-ring ${isHolder(p.id) ? "holder" : ""}`} />
                    {p.id} {p.id === status.selfId ? "(me)" : ""}
                  </span>
                }
                extra={
                  <Space>
                    <Switch
                      size="small"
                      unCheckedChildren={<AudioMutedOutlined />}
                      checkedChildren={<AudioOutlined />}
                      onChange={(on) =>
                        sessionAction(key, { type: "mute_set", peer: p.id, on: !on })
                      }
                    />
                    <Slider style={{ width: 90 }} defaultValue={80} min={0} max={100} />
                    {!isHolder(p.id) && (
                      <Button
                        size="small"
                        type="primary"
                        danger
                        icon={<CrownOutlined />}
                        onClick={() => sessionAction(key, { type: "floor_request", priority: 9, preempt: true })}
                      >
                        Preempt
                      </Button>
                    )}
                  </Space>
                }
              >
                <Text type="secondary">role: {p.role} · since {p.connected_since_ms}</Text>
              </Card>
            ))}
            <div className="mirror-note">tally: red ring = floor holder</div>
          </Card>
        </Col>

        {/* queue strip + event log */}
        <Col span={10}>
          <Card title={`Grant queue (${mirror.queue.length})`} size="small" style={{ marginBottom: 12 }}>
            {mirror.queue.length === 0 && <Text type="secondary">empty</Text>}
            {mirror.queue.map((q, i) => (
              <Tag key={`${q.peer}-${i}`} color="orange" style={{ marginBottom: 4 }}>
                #{i + 1} {q.peer} · p{q.priority}
              </Tag>
            ))}
            {meHolder && <div className="mirror-note">you hold the floor</div>}
          </Card>

          <Card title="Event log" size="small">
            <List
              size="small"
              dataSource={status.events}
              locale={{ emptyText: "no events" }}
              renderItem={(e) => (
                <List.Item>
                  {e.kind === "denied" && <Text type="warning">denied: {e.reason}</Text>}
                  {e.kind === "taken" && <Text type="danger">floor taken by {e.by}</Text>}
                  {e.kind === "error" && <Text type="danger">error: {e.code}</Text>}
                </List.Item>
              )}
            />
          </Card>
        </Col>
      </Row>
    </div>
  );
}

/** Fire-and-forget authorized action through the session's client. */
function sessionAction(
  key: string,
  msg:
    | { type: "mode_change"; mode: "exclusive" | "open" | "hybrid" }
    | { type: "mute_set"; peer: string; on: boolean }
    | { type: "floor_release" }
    | { type: "floor_request"; priority: number; preempt: boolean },
) {
  const session = getSession(key);
  if (!session) return;
  const { client } = session;
  if (msg.type === "mode_change") client.changeMode(msg.mode);
  else if (msg.type === "mute_set") client.mutePeer(msg.peer, msg.on);
  else if (msg.type === "floor_release") client.releaseFloor();
  else client.requestFloor(msg.priority, msg.preempt);
}
