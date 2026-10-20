// Intervention kit UI (plan-7 S6, D23 R2): the per-room peer table plus the
// kick / force-release confirm Modal (optional reason = AUDIT ONLY, never
// broadcast) and the peer detail Drawer fed by the enriched live report (6.2).
// Split out of monitor.tsx so the poll/view half stays under the plan-7 150-line bar.
import React, { useState } from "react";
import { Button, Descriptions, Drawer, Input, Modal, Space, Table, Tag } from "antd";
import type { AdminApi, PeerRow, RoomRow } from "../../lib/adminApi.js";

interface Selection {
  room: string;
  peer: PeerRow;
}
interface Action extends Selection {
  kind: "kick" | "release";
}

export function PeerPanel({
  room,
  api,
  onError,
}: {
  room: RoomRow;
  api: AdminApi;
  onError: (message: string) => void;
}): React.ReactElement {
  const [action, setAction] = useState<Action | null>(null);
  const [reason, setReason] = useState("");
  const [drawer, setDrawer] = useState<Selection | null>(null);

  const runAction = async () => {
    if (!action) return;
    const { kind, room: name, peer } = action;
    try {
      if (kind === "kick") await api.kickPeer(name, peer.id, reason || undefined);
      else await api.forceRelease(name, peer.id, reason || undefined);
    } catch (e) {
      onError((e as Error).message);
    }
    setAction(null);
    setReason("");
  };

  return (
    <>
      <Table<PeerRow>
        rowKey="id"
        size="small"
        dataSource={room.peers ?? []}
        pagination={false}
        columns={[
          {
            title: "Peer",
            key: "id",
            render: (_, p) => (
              <Button
                type="link"
                size="small"
                style={{ padding: 0 }}
                onClick={() => setDrawer({ room: room.room, peer: p })}
              >
                {p.id}
              </Button>
            ),
          },
          { title: "Role", dataIndex: "role", key: "role" },
          {
            title: "State",
            key: "state",
            render: (_, p) => (
              <Space size={4}>
                {p.granted && <Tag color="red">holding</Tag>}
                {p.muted && <Tag color="volcano">muted</Tag>}
                {(p.mod_rooms ?? []).map((m) => (
                  <Tag key={m} color="blue">
                    mod:{m}
                  </Tag>
                ))}
              </Space>
            ),
          },
          {
            title: "Actions",
            key: "actions",
            render: (_, p) => (
              <Space size={4}>
                <Button size="small" onClick={() => setAction({ kind: "release", room: room.room, peer: p })}>
                  Force release
                </Button>
                <Button size="small" danger onClick={() => setAction({ kind: "kick", room: room.room, peer: p })}>
                  Kick
                </Button>
              </Space>
            ),
          },
        ]}
      />
      <Modal
        open={action !== null}
        title={
          action
            ? `${action.kind === "kick" ? "Kick" : "Force-release"} ${action.peer.id} from ${action.room}?`
            : ""
        }
        okText={action?.kind === "kick" ? "Kick" : "Recall floor"}
        okButtonProps={{ danger: action?.kind === "kick" }}
        onOk={() => void runAction()}
        onCancel={() => {
          setAction(null);
          setReason("");
        }}
      >
        <Input
          placeholder="Reason (optional; recorded in the audit log only)"
          value={reason}
          onChange={(e) => setReason(e.target.value)}
          maxLength={512}
        />
      </Modal>
      <Drawer
        open={drawer !== null}
        onClose={() => setDrawer(null)}
        title={drawer ? `${drawer.room} / ${drawer.peer.id}` : ""}
      >
        {drawer && (
          <Descriptions column={1} size="small">
            <Descriptions.Item label="role">{drawer.peer.role ?? "—"}</Descriptions.Item>
            <Descriptions.Item label="holding floor">{String(drawer.peer.granted ?? false)}</Descriptions.Item>
            <Descriptions.Item label="muted">{String(drawer.peer.muted ?? false)}</Descriptions.Item>
            <Descriptions.Item label="moderator of">
              {(drawer.peer.mod_rooms ?? []).join(", ") || "—"}
            </Descriptions.Item>
          </Descriptions>
        )}
      </Drawer>
    </>
  );
}
