// Logs tab (plan-7 S1 audit face): newest-first page over /admin/api/audit
// with event/room/actor filters and a manual refresh (no auto-poll - audit is
// an evidence surface, not telemetry). `dropped` warns loudly when nonzero (R4).
import React, { useCallback, useEffect, useRef, useState } from "react";
import { Alert, Button, Input, Select, Space, Table, Tag } from "antd";
import type { AdminApi, AuditRow } from "../../lib/adminApi.js";
import { LOGS_DEBOUNCE_MS, type Link } from "../../lib/adminEventsClient.js";

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

export function LogsTab({
  api,
  hint,
  link,
}: {
  api: AdminApi;
  hint: number;
  link: Link;
}): React.ReactElement {
  const [rows, setRows] = useState<AuditRow[]>([]);
  const [dropped, setDropped] = useState(0);
  const [error, setError] = useState<string | null>(null);
  const [event, setEvent] = useState<string | undefined>();
  const [room, setRoom] = useState("");
  const [actor, setActor] = useState("");

  // highest seq currently on screen: the cursor for the incremental fetches that
  // hints trigger. A hint NEVER carries a row (plan-9 R3=B), so `refresh` and its
  // after-cursor remain the only paths by which a line can appear here
  const maxSeq = useRef(0);
  const seenHint = useRef(0);
  // latest hint value, readable from a callback that must NOT depend on it: putting
  // `hint` in refresh's deps made every hint re-fire the full-page fetch, which
  // quietly defeats the debounce below (and made the burst test pass by accident)
  const hintRef = useRef(0);

  const refresh = useCallback(() => {
    api
      .audit({ event, room: room || undefined, actor: actor || undefined, limit: 100 })
      .then((p) => {
        setRows(p.events);
        setDropped(p.dropped);
        setError(null);
        maxSeq.current = p.events.reduce((m, r) => Math.max(m, r.seq), 0);
        // filters just changed: the old cursor belongs to the old query
        seenHint.current = hintRef.current;
      })
      .catch((e: Error) => setError(e.message));
  }, [api, event, room, actor]);

  // hint -> ONE debounced incremental fetch per burst (LOGS_DEBOUNCE_MS). Without
  // the debounce a 30-peer join wave becomes 30 fetches, which is worse than the
  // poll it replaced
  useEffect(() => {
    hintRef.current = hint;
    if (hint === seenHint.current) return;
    seenHint.current = hint;
    const after = maxSeq.current;
    const t = setTimeout(() => {
      api
        .audit({
          event,
          room: room || undefined,
          actor: actor || undefined,
          after,
          limit: 100,
        })
        .then((p) => {
          if (!p.events.length) return;
          setDropped(p.dropped);
          setError(null);
          maxSeq.current = p.events.reduce(
            (m, r) => Math.max(m, r.seq),
            maxSeq.current,
          );
          setRows((cur) => {
            const bySeq = new Map<number, AuditRow>();
            for (const r of [...cur, ...p.events]) bySeq.set(r.seq, r);
            return [...bySeq.values()].sort((a, b) => b.seq - a.seq).slice(0, 100);
          });
        })
        .catch((e: Error) => setError(e.message));
    }, LOGS_DEBOUNCE_MS);
    return () => clearTimeout(t);
  }, [hint, api, event, room, actor]);

  // Re-subscribing is not the same as catching up: a link that drops and returns may
  // have missed hints, so ask for one full page then. NOT on the first hello — the
  // mount refresh already covers first paint, and treating it as a reconnect fetched
  // the whole page twice on every open (caught by the burst test).
  const everLive = useRef(false);
  useEffect(() => {
    if (link !== "live") return;
    if (everLive.current) refresh();
    everLive.current = true;
  }, [link, refresh]);

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
