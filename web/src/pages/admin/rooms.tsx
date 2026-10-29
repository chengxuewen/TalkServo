// Rooms tab (plan-7 S2 registry UI, S7 split): rooms.yaml CRUD table + form.
// Operator tier: read + create/edit rows is admin-only per S5 - writes are
// disabled here and refused by the server (403). Delete warns the kick-all
// consequence (S2 eviction).
import React, { useEffect, useState } from "react";
import { Alert, Button, Card, Form, Input, InputNumber, Modal, Popconfirm, Select, Space, Switch, Table } from "antd";
import { EntryActions } from "./entryActions.js";
import type { AdminApi, RoomConfigRow } from "../../lib/adminApi.js";

type Draft = Partial<RoomConfigRow> & { room: string };

export function RoomsTab({ api, tier }: { api: AdminApi; tier: string }): React.ReactElement {
  const [rows, setRows] = useState<RoomConfigRow[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [draft, setDraft] = useState<Draft | null>(null);
  // The Modal's OK button sits OUTSIDE the Form, so AntD never runs onFinish and the
  // typed values used to vanish: save() posted the raw draft (an EMPTY room id for
  // "New room"), the server answered 400, and the registry looked un-writable. The
  // e2e registry row found it; the vitest row only checked the button existed.
  const [form] = Form.useForm<Draft>();

  const openDraft = (d: Draft) => {
    // the form store outlives the fields, so seeding here survives the modal opening
    // on the next render and clears whatever the previous edit left behind
    form.resetFields();
    form.setFieldsValue(d);
    setDraft(d);
  };

  const reload = () =>
    api
      .roomsRegistry()
      .then(setRows)
      .catch((e: Error) => setError(e.message));
  useEffect(() => {
    reload();
  }, [api]);

  const save = async () => {
    if (!draft) return;
    let values: Draft;
    try {
      values = await form.validateFields(); // a failed rule keeps the modal open
    } catch {
      return;
    }
    try {
      await api.createRoom(
        { ...draft, ...values } as { room: string } & Partial<RoomConfigRow>,
      );
      setDraft(null);
      setError(null);
      reload();
    } catch (e) {
      setError((e as Error).message);
    }
  };

  const remove = async (room: string) => {
    try {
      await api.deleteRoom(room);
      setError(null);
      reload();
    } catch (e) {
      setError((e as Error).message);
    }
  };

  const writable = tier === "admin";

  return (
    <Card title="Room registry (rooms.yaml)" style={{ maxWidth: 860 }}>
      {error && <Alert type="error" message={error} style={{ marginBottom: 12 }} />}
      {!writable && (
        <Alert type="info" message="operator tier: registry writes require admin" style={{ marginBottom: 12 }} />
      )}
      <Button
        type="primary"
        disabled={!writable}
        onClick={() => openDraft({ room: "", owner: "", locked: false })}
      >
        New room
      </Button>
      <Table<RoomConfigRow>
        rowKey="room"
        size="small"
        style={{ marginTop: 12 }}
        dataSource={rows}
        pagination={false}
        columns={[
          { title: "Room", dataIndex: "room", key: "room" },
          { title: "Owner", dataIndex: "owner", key: "owner" },
          { title: "Mode", key: "mode", render: (_, r) => r.mode ?? "hybrid (default)" },
          { title: "Max peers", key: "mp", render: (_, r) => r.max_peers ?? "—" },
          { title: "Locked", key: "lk", render: (_, r) => (r.locked ? "🔒" : "—") },
          {
            title: "",
            key: "ops",
            render: (_, r) => (
              <span>
                <EntryActions api={api} room={r.room} />
                <Button size="small" disabled={!writable} style={{ marginLeft: 6 }} onClick={() => openDraft({ ...r })}>
                  Edit
                </Button>
                <Popconfirm
                  title={`Delete ${r.room}?`}
                  description="Removes the registry row and kicks every live member."
                  disabled={!writable}
                  onConfirm={() => void remove(r.room)}
                >
                  <Button size="small" danger disabled={!writable} style={{ marginLeft: 6 }}>
                    Delete
                  </Button>
                </Popconfirm>
              </span>
            ),
          },
        ]}
      />
      <Modal open={draft !== null} okText="Save" onOk={() => void save()} onCancel={() => setDraft(null)}>
        {draft && (
          <Form form={form} layout="vertical" initialValues={draft}>
            <Form.Item name="room" label="Room id" rules={[{ required: true }]}>
              <Input placeholder="ops-01" disabled={!!rows.find((r) => r.room === draft.room)} />
            </Form.Item>
            <Form.Item name="owner" label="Owner (label)">
              <Input placeholder="squad-a" />
            </Form.Item>
            <Form.Item name="mode" label="Default mode">
              <Select
                allowClear
                placeholder="hybrid"
                options={[
                  { value: "exclusive", label: "exclusive" },
                  { value: "open", label: "open" },
                  { value: "hybrid", label: "hybrid" },
                ]}
              />
            </Form.Item>
            <Form.Item name="max_peers" label="Max peers (empty = global)">
              <InputNumber min={1} max={1024} style={{ width: "100%" }} />
            </Form.Item>
            <Form.Item name="locked" label="Locked (refuse joins)" valuePropName="checked">
              <Switch />
            </Form.Item>
          </Form>
        )}
      </Modal>
    </Card>
  );
}
