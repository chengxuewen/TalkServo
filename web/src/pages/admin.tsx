// Admin console (plan-5 S2, D18): out-of-band /admin/api consumer. Gate
// card -> sessionStorage JWT -> Monitor (3s poll, one fan-out per tick) +
// Tokens (issue form -> copy-ready join links). Wire truth stays in
// talkservo-server/src/admin.rs; this page renders it verbatim, including
// per-room `stale` marks from the 1s fan-out timeout.

import React, { useEffect, useMemo, useState } from "react";
import {
  Alert,
  Button,
  Card,
  Form,
  Input,
  InputNumber,
  Layout,
  Select,
  Space,
  Table,
  Tag,
  Typography,
} from "antd";
import {
  AdminApi,
  adminJwt,
  setAdminJwt,
  type Issued,
  type Overview,
  type RoomRow,
} from "../lib/adminApi.js";

const { Sider, Content } = Layout;

export function AdminPage(): React.ReactElement {
  const [authorized, setAuthorized] = useState<boolean>(() => !!adminJwt());
  const [tokenDraft, setTokenDraft] = useState("");
  const [gateError, setGateError] = useState<string | null>(null);
  const [tab, setTab] = useState<"monitor" | "tokens">("monitor");
  const [overview, setOverview] = useState<Overview | null>(null);
  const [rooms, setRooms] = useState<RoomRow[]>([]);
  const [pollError, setPollError] = useState<string | null>(null);
  const [issued, setIssued] = useState<Issued | null>(null);
  const [issueError, setIssueError] = useState<string | null>(null);

  const api = useMemo(
    () =>
      new AdminApi(() => {
        setAdminJwt(null);
        setAuthorized(false);
        setGateError("rejected — paste a valid admin JWT (503 = surface disabled)");
      }),
    [],
  );

  const enter = async () => {
    setAdminJwt(tokenDraft.trim());
    try {
      await api.overview();
      setGateError(null);
      setAuthorized(true);
    } catch {
      setAdminJwt(null); // 401/503 bounced by the api callback already
    }
  };

  useEffect(() => {
    if (!authorized || tab !== "monitor") return;
    let stop = false;
    const tick = async () => {
      try {
        const [o, r] = await Promise.all([api.overview(), api.rooms()]);
        if (!stop) {
          setOverview(o);
          setRooms(r);
          setPollError(null);
        }
      } catch (e) {
        if (!stop) setPollError((e as Error).message);
      }
    };
    void tick();
    const iv = setInterval(tick, 3000); // 3s poll (plan-5; WS push = Alpha)
    return () => {
      stop = true;
      clearInterval(iv);
    };
  }, [authorized, tab, api]);

  if (!authorized) {
    return (
      <Layout style={{ minHeight: "100vh" }}>
        <Content style={{ display: "grid", placeItems: "center" }}>
          <Card title="Admin access" style={{ width: 420 }}>
            <Space direction="vertical" style={{ width: "100%" }}>
              <Input.Password
                placeholder="Admin JWT"
                value={tokenDraft}
                onChange={(e) => setTokenDraft(e.target.value)}
                onPressEnter={() => void enter()}
              />
              <Button type="primary" onClick={() => void enter()}>
                Enter
              </Button>
              {gateError && <Alert type="error" message={gateError} />}
            </Space>
          </Card>
        </Content>
      </Layout>
    );
  }

  const copy = (text: string) => {
    void navigator.clipboard?.writeText(text);
  };

  const columns = [
    { title: "Room", dataIndex: "room", key: "room" },
    {
      title: "Peers",
      key: "peers",
      render: (_: unknown, r: RoomRow) => (r.stale ? "—" : (r.peers?.length ?? 0)),
    },
    {
      title: "Floor",
      key: "holders",
      render: (_: unknown, r: RoomRow) =>
        r.stale ? (
          <Tag color="orange">stale</Tag>
        ) : (
          (r.holders ?? []).map((h) => (
            <span key={h} style={{ marginRight: 8 }}>
              <span className="tally-ring holder" /> {h}
              {(r.speaking ?? []).includes(h) ? (
                <Tag color="green" style={{ marginLeft: 4 }}>
                  speaking
                </Tag>
              ) : (
                <Tag style={{ marginLeft: 4 }}>silent</Tag>
              )}
            </span>
          ))
        ),
    },
    {
      title: "Queue",
      key: "queue",
      render: (_: unknown, r: RoomRow) => (r.stale ? "—" : r.queue_len ?? 0),
    },
    {
      title: "Muted",
      key: "muted",
      render: (_: unknown, r: RoomRow) =>
        (r.muted ?? []).map((m) => (
          <Tag key={m} color="volcano">
            {m}
          </Tag>
        )),
    },
    {
      title: "Mode",
      key: "mode",
      render: (_: unknown, r: RoomRow) => r.mode ?? "—",
    },
  ];

  return (
    <Layout style={{ minHeight: "100vh" }}>
      <Sider width={180} theme="dark">
        <div style={{ padding: 16 }}>
          <Typography.Text strong style={{ color: "#fff" }}>
            TalkServo
          </Typography.Text>
        </div>
        <Space direction="vertical" style={{ width: "100%", padding: "0 8px" }}>
          <Button block type={tab === "monitor" ? "primary" : "default"} onClick={() => setTab("monitor")}>
            Monitor
          </Button>
          <Button block type={tab === "tokens" ? "primary" : "default"} onClick={() => setTab("tokens")}>
            Tokens
          </Button>
        </Space>
      </Sider>
      <Content style={{ padding: 24 }}>
        {tab === "monitor" ? (
          <>
            {pollError && <Alert type="warning" message={pollError} style={{ marginBottom: 12 }} />}
            <Space style={{ marginBottom: 16 }}>
              <Card size="small">Rooms {overview?.rooms ?? "—"}</Card>
              <Card size="small">Peers {overview?.peers ?? "—"}</Card>
              <Card size="small">Up {overview?.uptime_s ?? "—"}s</Card>
              <Card size="small">Backend {overview?.backend ?? "—"}</Card>
            </Space>
            <Table
              rowKey="room"
              size="small"
              dataSource={rooms}
              columns={columns}
              pagination={false}
            />
          </>
        ) : (
          <Card title="Issue join token" style={{ maxWidth: 560 }}>
            <Form
              layout="vertical"
              initialValues={{ role: "field", ttl_s: 3600 }}
              onFinish={(v: { sub: string; room: string; role: "dispatch" | "field"; ttl_s?: number }) => {
                setIssueError(null);
                api
                  .issue(v)
                  .then(setIssued)
                  .catch((e: Error) => setIssueError(e.message));
              }}
            >
              <Form.Item name="sub" label="Subject (device/user id)" rules={[{ required: true }]}>
                <Input placeholder="field-7" />
              </Form.Item>
              <Form.Item name="room" label="Room" rules={[{ required: true }]}>
                <Input placeholder="ops-01" />
              </Form.Item>
              <Form.Item name="role" label="Role">
                <Select
                  options={[
                    { value: "field", label: "field" },
                    { value: "dispatch", label: "dispatch" },
                  ]}
                />
              </Form.Item>
              <Form.Item name="ttl_s" label="TTL (s)">
                <InputNumber min={1} style={{ width: "100%" }} />
              </Form.Item>
              <Button type="primary" htmlType="submit">
                Issue
              </Button>
              {issueError && <Alert type="error" message={issueError} style={{ marginTop: 12 }} />}
            </Form>
            {issued && (
              <Card size="small" title="Links" style={{ marginTop: 16 }}>
                {(
                  [
                    ["field", issued.field_url],
                    ["dispatch", issued.dispatch_url],
                    ["jwt", issued.jwt],
                  ] as const
                ).map(([label, value]) => (
                  <Space key={label} style={{ display: "flex", marginBottom: 8 }}>
                    <Typography.Text copyable={{ text: value }} style={{ maxWidth: 420 }}>
                      {label}: {value.length > 60 ? `${value.slice(0, 60)}…` : value}
                    </Typography.Text>
                    <Button size="small" onClick={() => copy(value)}>
                      Copy
                    </Button>
                  </Space>
                ))}
              </Card>
            )}
          </Card>
        )}
      </Content>
    </Layout>
  );
}
