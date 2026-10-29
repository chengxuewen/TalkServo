// EntryActions (entry-flow wave, D29-#3/#5): row-level Enter + Copy links for the
// Rooms registry table. Enter mints the operator's OWN dispatch token via the issue
// endpoint with an EMPTY sub (server generates u-xxxxxx, fixed at issue) and opens the
// dispatch join URL; Copy links puts all three join URLs (field/dispatch/participant)
// into the clipboard as plain text, newline-joined — channel-agnostic paste-to-group.
import React from "react";
import { Button, Space, App } from "antd";
import type { AdminApi } from "../../lib/adminApi.js";

/** Clipboard with textarea fallback (navigator.clipboard needs a secure context — the
 *  LAN flow is plain HTTP on a raw IP, so copy() alone would silently fail there). */
async function copyText(text: string): Promise<void> {
  if (navigator.clipboard?.writeText) {
    await navigator.clipboard.writeText(text);
    return;
  }
  const ta = document.createElement("textarea");
  ta.value = text;
  ta.style.position = "fixed";
  ta.style.opacity = "0";
  document.body.appendChild(ta);
  ta.select();
  document.execCommand("copy");
  document.body.removeChild(ta);
}

export function EntryActions({ api, room }: { api: AdminApi; room: string }): React.ReactElement {
  const { message } = App.useApp();

  const enter = async () => {
    try {
      // sub OMITTED on purpose: the server generates u-xxxxxx (D29-#4) — the test
      // asserts its ABSENCE on the wire (ratchet), the server asserts the format.
      const issued = await api.issue({ room, role: "dispatch" });
      window.open(issued.dispatch_url, "_blank", "noopener");
    } catch (e) {
      message.error(`enter failed: ${(e as Error).message}`);
    }
  };

  const copyLinks = async () => {
    try {
      const issued = await api.issue({ room, role: "field" });
      const text = [
        issued.field_url,
        issued.dispatch_url,
        issued.participant_url,
      ].join("\n");
      await copyText(text);
      message.success(`links for ${room} copied (3)`);
    } catch (e) {
      message.error(`copy failed: ${(e as Error).message}`);
    }
  };

  return (
    <Space size={4}>
      <Button size="small" onClick={enter}>
        Enter
      </Button>
      <Button size="small" onClick={copyLinks}>
        Copy links
      </Button>
    </Space>
  );
}
