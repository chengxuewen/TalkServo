// Tokens tab (plan-7 S4 UI): issue join links, optionally granting room
// moderator claims (mod_rooms multi-select fed by the rooms registry).
import React, { useEffect, useState } from "react";
import { Alert, Button, Card, Form, Input, InputNumber, Select, Space, Typography } from "antd";
import type { AdminApi, Issued, RoomConfigRow } from "../../lib/adminApi.js";

interface Draft {
  sub: string;
  room: string;
  role: "dispatch" | "field";
  ttl_s?: number;
  mod_rooms?: string[];
}

export function TokensTab({ api }: { api: AdminApi }): React.ReactElement {
  const [issued, setIssued] = useState<Issued | null>(null);
  const [issueError, setIssueError] = useState<string | null>(null);
  const [knownRooms, setKnownRooms] = useState<string[]>([]);

  useEffect(() => {
    api
      .roomsRegistry()
      .then((rows: RoomConfigRow[]) => setKnownRooms(rows.map((r) => r.room)))
      .catch(() => setKnownRooms([])); // registry optional - multi-select stays empty
  }, [api]);

  const copy = (text: string) => {
    void navigator.clipboard?.writeText(text);
  };

  return (
    <Space direction="vertical" style={{ width: "100%", maxWidth: 560 }}>
      <Card title="Issue join token">
        <Form
          layout="vertical"
          initialValues={{ role: "field", ttl_s: 3600 }}
          onFinish={(v: Draft) => {
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
          <Form.Item name="role" label="Signaling role">
            <Select
              options={[
                { value: "field", label: "field" },
                { value: "dispatch", label: "dispatch" },
              ]}
            />
          </Form.Item>
          <Form.Item name="mod_rooms" label="Moderator of (S4 claims)" tooltip="Grants room-command authority (mute/kick/mode) for the listed rooms">
            <Select mode="multiple" allowClear placeholder={knownRooms.length ? "claims-free by default" : "no registry rooms yet"} options={knownRooms.map((r) => ({ value: r, label: r }))} />
          </Form.Item>
          <Form.Item name="ttl_s" label="TTL (s)">
            <InputNumber min={1} style={{ width: "100%" }} />
          </Form.Item>
          <Button type="primary" htmlType="submit">
            Issue
          </Button>
          {issueError && <Alert type="error" message={issueError} style={{ marginTop: 12 }} />}
        </Form>
      </Card>
      {issued && (
        <Card size="small" title="Links">
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
    </Space>
  );
}
