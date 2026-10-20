// Accounts tab (admin tier only): registry table + create/reset form with the
// S5 role dropdown (create-only - reset keeps the stored tier, server rule).
import React, { useEffect, useState } from "react";
import { Alert, Button, Card, Form, Input, Popconfirm, Select, Space, Table } from "antd";
import type { AccountRow, AdminApi } from "../../lib/adminApi.js";

export function AccountsTab({ api }: { api: AdminApi }): React.ReactElement {
  const [accounts, setAccounts] = useState<AccountRow[]>([]);
  const [error, setError] = useState<string | null>(null);

  const reload = () =>
    api
      .accounts()
      .then(setAccounts)
      .catch((e: Error) => setError(e.message));
  useEffect(() => {
    reload();
  }, [api]);

  const remove = async (user: string) => {
    setError(null);
    try {
      await api.removeAccount(user);
      reload();
    } catch (e) {
      setError((e as Error).message);
    }
  };

  return (
    <Space direction="vertical" style={{ width: "100%", maxWidth: 560 }}>
      <Card title="Accounts">
        <Table<AccountRow>
          rowKey="user"
          size="small"
          dataSource={accounts}
          pagination={false}
          columns={[
            { title: "User", dataIndex: "user", key: "user" },
            { title: "Role", dataIndex: "role", key: "role" },
            {
              title: "",
              key: "actions",
              render: (_, a) => (
                <Popconfirm title={`Remove ${a.user}?`} onConfirm={() => void remove(a.user)}>
                  <Button size="small" danger>
                    Remove
                  </Button>
                </Popconfirm>
              ),
            },
          ]}
        />
      </Card>
      <Card title="Create account / reset password">
        <Form
          layout="vertical"
          initialValues={{ role: "operator" }}
          onFinish={(v: { user: string; password: string; role: string }) => {
            setError(null);
            api
              .createAccount(v.user, v.password, v.role)
              .then(reload)
              .catch((e: Error) => setError(e.message));
          }}
        >
          <Form.Item name="user" label="Username" rules={[{ required: true }]}>
            <Input placeholder="ops-1" />
          </Form.Item>
          <Form.Item name="password" label="Password (min 8 chars)" rules={[{ required: true, min: 8 }]}>
            <Input.Password />
          </Form.Item>
          <Form.Item name="role" label="Tier (new accounts; resets keep the stored tier)">
            <Select
              options={[
                { value: "operator", label: "operator" },
                { value: "admin", label: "admin" },
              ]}
            />
          </Form.Item>
          <Button type="primary" htmlType="submit">
            Save
          </Button>
          {error && <Alert type="error" message={error} style={{ marginTop: 12 }} />}
        </Form>
      </Card>
    </Space>
  );
}
