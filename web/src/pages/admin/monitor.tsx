// Monitor tab (plan-7 S7): 3s live poll + rooms table; the peer half (expand
// rows, intervention kit, detail drawer) lives in PeerPanel (own file).
import React, { useEffect, useState } from "react";
import { Alert, Card, Space, Table, Tag } from "antd";
import type { AdminApi, Overview, RoomRow } from "../../lib/adminApi.js";
import { PeerPanel } from "./intervention.js";

export function MonitorTab({ api }: { api: AdminApi }): React.ReactElement {
  const [overview, setOverview] = useState<Overview | null>(null);
  const [rooms, setRooms] = useState<RoomRow[]>([]);
  const [pollError, setPollError] = useState<string | null>(null);

  useEffect(() => {
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
    const iv = setInterval(tick, 3000);
    return () => {
      stop = true;
      clearInterval(iv);
    };
  }, [api]);

  return (
    <>
      {pollError && <Alert type="warning" message={pollError} style={{ marginBottom: 12 }} />}
      <Space style={{ marginBottom: 16 }} wrap>
        <Card size="small">Rooms {overview?.rooms ?? "—"}</Card>
        <Card size="small">Peers {overview?.peers ?? "—"}</Card>
        <Card size="small">Up {overview?.uptime_s ?? "—"}s</Card>
        <Card size="small">Backend {overview?.backend ?? "—"}</Card>
      </Space>
      <Table<RoomRow>
        rowKey="room"
        size="small"
        dataSource={rooms}
        pagination={false}
        expandable={{ expandedRowRender: (r) => <PeerPanel room={r} api={api} onError={setPollError} /> }}
        columns={[
          { title: "Room", dataIndex: "room", key: "room" },
          {
            title: "Peers",
            key: "peers",
            render: (_, r) => (r.stale ? "—" : (r.peers?.length ?? 0)),
          },
          {
            title: "Floor",
            key: "holders",
            render: (_, r) =>
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
            title: "Muted",
            key: "muted",
            render: (_, r) =>
              (r.muted ?? []).map((m) => (
                <Tag key={m} color="volcano">
                  {m}
                </Tag>
              )),
          },
          { title: "Queue", key: "queue", render: (_, r) => (r.stale ? "—" : r.queue_len ?? 0) },
          { title: "Mode", key: "mode", render: (_, r) => r.mode ?? "—" },
        ]}
      />
    </>
  );
}
