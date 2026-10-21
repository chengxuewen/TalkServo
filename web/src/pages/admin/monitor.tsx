// Monitor tab (plan-7 S7): rooms table + the peer half (expand rows, intervention
// kit, detail drawer, all in PeerPanel). Plan-9 W2: the 3s poll became a 30s
// RECONCILE under push, tightening back to 3s whenever the socket is not live — the
// poll is also this tab's only tested error path, so dropping it would trade a
// visible failure mode for an invisible one (plan-9 R2=A).
import React, { useEffect, useState } from "react";
import { Alert, Card, Space, Table, Tag, Typography } from "antd";
import type { AdminApi, Overview, RoomRow } from "../../lib/adminApi.js";
import { PeerPanel } from "./intervention.js";
import { FALLBACK_MS, RECONCILE_MS, type Link } from "../../lib/adminEventsClient.js";

const { Text } = Typography;

export function MonitorTab({
  api,
  link,
  snapshot,
}: {
  api: AdminApi;
  link: Link;
  snapshot: RoomRow[] | null;
}): React.ReactElement {
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
    const iv = setInterval(tick, link === "live" ? RECONCILE_MS : FALLBACK_MS);
    return () => {
      stop = true;
      clearInterval(iv);
    };
  }, [api, link]);

  // push wins the race because it is applied on arrival; the reconcile above only
  // ever re-confirms it, so a lost RoomsLive is a delay and not a lie
  useEffect(() => {
    if (snapshot) setRooms(snapshot);
  }, [snapshot]);

  return (
    <>
      {pollError && <Alert type="warning" message={pollError} style={{ marginBottom: 12 }} />}
      {link !== "live" && (
        <div style={{ marginBottom: 10 }}>
          <Text type="secondary" data-testid="push-state">
            live push {link} — reconciling every {Math.round(FALLBACK_MS / 1000)}s
          </Text>
        </div>
      )}
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
