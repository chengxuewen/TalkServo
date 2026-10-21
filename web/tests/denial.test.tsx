// S7.5 (plan-7 D23 R3 + A1/A2/A3 walkthrough): "denied" and "muted" must read as
// two different things on the speaking surfaces. The wire deliberately shares the
// `unauthorized` reason between the muted-request refusal (Rule 2) and the
// command-authorization refusal (Rule 10) — D16 compat forbids a new variant —
// so the mirrored roster is the only thing that tells them apart. That is a UI
// contract, and this file pins it.
import { describe, it, expect, afterEach } from "vitest";
import { render, screen, cleanup } from "@testing-library/react";
import { MemoryRouter, Route, Routes } from "react-router-dom";
import { ConfigProvider, theme } from "antd";
import React from "react";
import { FieldPage } from "../src/pages/field.js";
import { ParticipantPage } from "../src/pages/participant.js";
import { ensureSession } from "../src/lib/session.js";
import { MUTED_BY_DISPATCHER, denialCopy } from "../src/lib/floorCopy.js";

/** Seed the S-3 session (one client per role:room) BEFORE the page renders, so
 *  the page's own ensureSession picks this instance up by the same room key. */
function seed(room: string, over: { muted?: string[]; denial?: string | null }) {
  const s = ensureSession({ room, role: "field" });
  s.connected = true;
  s.selfId = "me";
  s.lastDenial = over.denial ?? null;
  s.mirror = {
    ...s.mirror,
    mode: "exclusive",
    selfId: "me",
    peers: [{ id: "me", role: "field", connected_since_ms: 0 }],
    grants: [],
    muted: over.muted ?? [],
    generation: 7,
  };
  return s;
}

function uiAt(path: string, node: React.ReactElement) {
  return render(
    <ConfigProvider theme={{ algorithm: theme.darkAlgorithm }}>
      <MemoryRouter initialEntries={[path]}>
        <Routes>
          <Route path="/field/:room" element={node} />
          <Route path="/room/:room" element={node} />
        </Routes>
      </MemoryRouter>
    </ConfigProvider>,
  );
}

// Same discipline as the admin suite: without this, the previous test's DOM is
// still on screen and queries match two pages at once.
afterEach(cleanup);

describe("denial copy (pure)", () => {
  it("reads the shared unauthorized reason as a mute ONLY when the roster says so", () => {
    expect(denialCopy("unauthorized", true)).toContain(MUTED_BY_DISPATCHER);
    const commandReading = denialCopy("unauthorized", false);
    expect(commandReading).not.toContain("Muted");
    expect(commandReading).toContain("command");
  });

  it("words every wire reason and never blanks on an unknown one", () => {
    expect(denialCopy("busy", false)).toBe("Floor busy — the queue is full");
    expect(denialCopy("preempt_priority", false)).toContain("strictly higher");
    expect(denialCopy("exceeds_ceiling", false)).toContain("ceiling");
    expect(denialCopy("rate_limited", false)).toContain("Too fast");
    expect(denialCopy("not_member", false)).toContain("not a member");
    expect(denialCopy("no_media", false)).toContain("media path");
    expect(denialCopy("from_the_future", false)).toBe("Refused (from_the_future)");
  });
});

describe("field page", () => {
  it("shows the standing mute and suppresses the refusal line (mute wins)", () => {
    seed("s75-muted", { muted: ["me"], denial: "unauthorized" });
    uiAt("/field/s75-muted", <FieldPage />);
    const state = screen.getByTestId("floor-state");
    expect(state.textContent).toContain(MUTED_BY_DISPATCHER);
    expect(state.textContent).not.toContain("Floor busy");
    // the roster tags MY own row with the dispatcher wording, not a bare "muted"
    expect(screen.getByText("muted by dispatcher")).toBeTruthy();
  });

  it("shows a plain refusal as a refusal, with no mute wording anywhere", () => {
    seed("s75-denied", { muted: [], denial: "busy" });
    uiAt("/field/s75-denied", <FieldPage />);
    expect(screen.getByTestId("floor-state").textContent).toContain(
      "Floor busy — the queue is full",
    );
    expect(screen.queryByText(MUTED_BY_DISPATCHER)).toBeNull();
  });
});

describe("removal (R7-b)", () => {
  it("a terminal removal outranks the mute and the refusal lines", () => {
    const s = seed("s75-removed", { muted: ["me"], denial: "busy" });
    s.removedReason = "kicked by the dispatcher";
    uiAt("/field/s75-removed", <FieldPage />);
    const state = screen.getByTestId("floor-state");
    expect(state.textContent).toContain("Removed by dispatcher");
    expect(state.textContent).toContain("kicked by the dispatcher");
    expect(state.textContent).not.toContain("Muted by dispatcher");
    expect(state.textContent).not.toContain("Floor busy");
    // the button says what it now is, instead of an offline promise to retry
    expect(screen.getByText("REMOVED BY DISPATCHER")).toBeTruthy();
  });
});

describe("participant page", () => {
  it("renames the talk toggle while muted, and says why it will be refused", () => {
    seed("s75-part", { muted: ["me"], denial: "unauthorized" });
    uiAt("/room/s75-part", <ParticipantPage />);
    expect(screen.getByText("MUTED BY DISPATCHER")).toBeTruthy();
    expect(screen.getByTestId("floor-state").textContent).toContain(
      "until it is lifted",
    );
  });
});
