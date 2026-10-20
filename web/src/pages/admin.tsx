// Admin console (plan-5 S2, D18; D22 user/pwd): out-of-band /admin/api
// consumer. Username+Password gate -> server-minted JWT (sessionStorage) ->
// Monitor (3s poll) + Tokens (join links) + Accounts (registry CRUD). Wire truth stays in
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
  Popconfirm,
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
  type AccountRow,
  type Issued,
  type Overview,
  type RoomRow,
} from "../lib/adminApi.js";

const { Sider, Content } = Layout;

export function AdminPage(): React.ReactElement {
  const [authorized, setAuthorized] = useState<boolean>(() => !!adminJwt());
  const [userDraft, setUserDraft] = useState("");
  const [pwDraft, setPwDraft] = useState("");
  const [gateError, setGateError] = useState<string | null>(null);
  const [tab, setTab] = useState<"monitor" | "tokens" | "accounts">("monitor");
  const [accounts, setAccounts] = useState<AccountRow[]>([]);
  const [accountError, setAccountError] = useState<string | null>(null);
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
        setGateError("session expired or rejected — log in again");
      }),
    [],
  );

  const loginByPassword = async () => {
    try {
      await api.login(userDraft, pwDraft);
      await api.overview(); // confirm the minted token authenticates
      setGateError(null);
      setAuthorized(true);
    } catch (e) {
      const msg = String(e);
      setGateError(
        msg.includes("503")
          ? "no admin credentials configured on this server yet"
          : msg.includes("401")
            ? "invalid username or password"
            : "login failed",
      );
    }
  };

  useEffect(() => {
    if (!authorized || tab !== "monitor") return;
    let stop = false;
    const tick = async () => {
      try {
        const [o, r] = await Promise.all([api.overview(), api.roomsLive()]);
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

  useEffect(() => {
    if (!authorized || tab !== "accounts") return;
    api
      .accounts()
      .then((rows) => setAccounts(rows))
      .catch((e: Error) => setAccountError(e.message));
  }, [authorized, tab, api]);

  const removeAccount = async (user: string) => {
    setAccountError(null);
    try {
      await api.removeAccount(user);
      setAccounts(await api.accounts());
    } catch (e) {
      setAccountError((e as Error).message);
    }
  };

  if (!authorized) {
    return (
      <Layout style={{ minHeight: "100vh" }}>
        <Content style={{ display: "grid", placeItems: "center" }}>
          <Card title="Admin access" style={{ width: 420 }}>
            <Space direction="vertical" style={{ width: "100%" }}>
              <Input
                placeholder="Username"
                value={userDraft}
                onChange={(e) => setUserDraft(e.target.value)}
              />
              <Input.Password
                placeholder="Password"
                value={pwDraft}
                onChange={(e) => setPwDraft(e.target.value)}
                onPressEnter={() => void loginByPassword()}
              />
              <Button block type="primary" onClick={() => void loginByPassword()}>
                Log in
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
          <Button block type={tab === "accounts" ? "primary" : "default"} onClick={() => setTab("accounts")}>
            Accounts
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
        ) : tab === "tokens" ? (
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
                    ["participant", issued.participant_url],
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
        ) : (
          <Space direction="vertical" style={{ width: "100%", maxWidth: 560 }}>
            <Card title="Accounts">
              <Table
                rowKey="user"
                size="small"
                dataSource={accounts}
                columns={[
                  { title: "User", dataIndex: "user", key: "user" },
                  { title: "Role", dataIndex: "role", key: "role" },
                  {
                    title: "",
                    key: "actions",
                    render: (_: unknown, a: AccountRow) => (
                      <Popconfirm title={`Remove ${a.user}?`} onConfirm={() => void removeAccount(a.user)}>
                        <Button size="small" danger>
                          Remove
                        </Button>
                      </Popconfirm>
                    ),
                  },
                ]}
                pagination={false}
              />
            </Card>
            <Card title="Create account / reset password">
              <Form
                layout="vertical"
                onFinish={(v: { user: string; password: string }) => {
                  setAccountError(null);
                  api
                    .createAccount(v.user, v.password)
                    .then(() => api.accounts().then(setAccounts))
                    .catch((e: Error) => setAccountError(e.message));
                }}
              >
                <Form.Item name="user" label="Username" rules={[{ required: true }]}>
                  <Input placeholder="ops-1" />
                </Form.Item>
                <Form.Item name="password" label="Password (min 8 chars)" rules={[{ required: true, min: 8 }]}>
                  <Input.Password />
                </Form.Item>
                <Button type="primary" htmlType="submit">
                  Save
                </Button>
                {accountError && <Alert type="error" message={accountError} style={{ marginTop: 12 }} />}
              </Form>
            </Card>
          </Space>
        )}
      </Content>
    </Layout>
  );
}
