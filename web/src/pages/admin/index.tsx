// Admin console shell (plan-7 S7): gate + role-aware Sider + tab switch.
// D23 UI annex: five tabs `Monitor | Rooms | Tokens | Accounts(admin) | Logs`;
// Accounts renders for the admin tier only - the server's 403 is the real gate,
// the Sider filter is UX (D24 audit note). Sections split per file (<=150 lines).
import React, { useMemo, useState } from "react";
import { Alert, Button, Card, Input, Layout, Space } from "antd";
import { AdminApi, adminJwt, adminRole, setAdminJwt } from "../../lib/adminApi.js";
import { MonitorTab } from "./monitor.js";
import { RoomsTab } from "./rooms.js";
import { TokensTab } from "./tokens.js";
import { AccountsTab } from "./accounts.js";
import { LogsTab } from "./logs.js";

const { Sider, Content } = Layout;

type Tab = "monitor" | "rooms" | "tokens" | "accounts" | "logs";

export function AdminPage(): React.ReactElement {
  const [authorized, setAuthorized] = useState<boolean>(() => !!adminJwt());
  const [tier, setTier] = useState<string>(() => adminRole() ?? "admin");
  const [userDraft, setUserDraft] = useState("");
  const [pwDraft, setPwDraft] = useState("");
  const [gateError, setGateError] = useState<string | null>(null);
  const [tab, setTab] = useState<Tab>("monitor");

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
      setTier(adminRole() ?? "admin");
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

  const nav = (key: Tab, label: string) => (
    <Button
      block
      type={tab === key ? "primary" : "default"}
      onClick={() => setTab(key)}
    >
      {label}
    </Button>
  );

  return (
    <Layout style={{ minHeight: "100vh" }}>
      <Sider width={180} theme="dark">
        <div style={{ padding: 16 }}>
          <span data-testid="admin-tier" style={{ color: "#888", fontSize: 12 }}>
            {tier}
          </span>
          <div style={{ color: "#fff", fontWeight: 600 }}>TalkServo</div>
        </div>
        <Space direction="vertical" style={{ width: "100%", padding: "0 8px" }}>
          {nav("monitor", "Monitor")}
          {nav("rooms", "Rooms")}
          {nav("tokens", "Tokens")}
          {tier === "admin" && nav("accounts", "Accounts")}
          {nav("logs", "Logs")}
        </Space>
      </Sider>
      <Content style={{ padding: 24 }}>
        {tab === "monitor" && <MonitorTab api={api} />}
        {tab === "rooms" && <RoomsTab api={api} tier={tier} />}
        {tab === "tokens" && <TokensTab api={api} />}
        {tab === "accounts" && <AccountsTab api={api} />}
        {tab === "logs" && <LogsTab api={api} />}
      </Content>
    </Layout>
  );
}
