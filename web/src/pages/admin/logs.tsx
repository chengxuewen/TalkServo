// Logs tab (plan-7 S1 audit face): newest-first page over /admin/api/audit
// with event/room/actor filters and a manual refresh (no auto-poll - audit is
// an evidence surface, not telemetry). `dropped` warns loudly when nonzero (R4).
import React, { useCallback, useEffect, useState } from "react";
import { Alert, Button, Input, Select, Space, Table, Tag } from "antd";
import type { AdminApi, AuditRow } from "../../lib/adminApi.js";

const EVENTS = [
  "floor_grant",
  "floor_taken",
  "floor_idle",
  "mode_change",
  "mute_set",
  "kick",
  "force_release",
  "authz_denied",
  "join",
  "leave",
  "token_issue",
  "login_fail",
];

export function LogsTab({ api }: { api: AdminApi }): React.ReactElement {
  const [rows, setRows] = useState<AuditRow[]>([]);
  const [dropped, setDropped] = useState(0);
  const [error, setError] = useState<string | null>(null);
  const [event, setEvent] = useState<string | undefined>();
  const [room, setRoom] = useState("");
  const [actor, setActor] = useState("");

  const refresh = useCallback(() => {
    api
      .audit({ event, room: room || undefined, actor: actor || undefined, limit: 100 })
      .then((p) => {
        setRows(p.events);
        setDropped(p.dropped);
        setError(null);
      })
      .catch((e: Error) => setError(e.message));
  }, [api, event, room, actor]);

  useEffect(() => {
    refresh();
  }, [refresh]);

  return (
    <div>
      <Space style={{ marginBottom: 12 }} wrap>
        <Select
          allowClear
          placeholder="event"
          style={{ width: 160 }}
          value={event}
          onChange={setEvent}
          options={EVENTS.map((e) => ({ value: e, label: e }))}
        />
        <Input placeholder="room" value={room} onChange={(e) => setRoom(e.target.value)} style={{ width: 140 }} />
        <Input placeholder="actor" value={actor} onChange={(e) => setActor(e.target.value)} style={{ width: 140 }} />
        <Button type="primary" onClick={refresh}>
          Refresh
        </Button>
      </Space>
      {error && <Alert type="error" message={error} style={{ marginBottom: 12 }} />}
      {dropped > 0 && (
        <Alert type="error" message={`${dropped} audit events failed to persist (see server log)`} style={{ marginBottom: 12 }} />
      )}
      <Table<AuditRow>
        rowKey="seq"
        size="small"
        dataSource={rows}
        pagination={false}
        columns={[
          { title: "Seq", dataIndex: "seq", key: "seq", width: 70 },
          {
            title: "Time",
            key: "ts",
            render: (_, r) => new Date(r.ts_ms).toISOString().slice(0, 19).replace("T", " "),
          },
          {
            title: "Event",
            dataIndex: "event",
            key: "event",
            render: (v: string) => (
              <Tag color={v === "authz_denied" || v === "login_fail" ? "red" : v.includes("floor") ? "blue" : undefined}>
                {v}
              </Tag>
            ),
          },
          { title: "Room", dataIndex: "room", key: "room" },
          { title: "Actor", dataIndex: "actor", key: "actor" },
          { title: "Target", dataIndex: "target", key: "target" },
          { title: "Reason", key: "reason", render: (_, r) => r.reason || "—" },
        ]}
      />
    </div>
  );
}
