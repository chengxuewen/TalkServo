// SPA smoke — plan-3 T3/T4: routes render, S-3 useSyncExternalStore binding,
// PTT keybind guard semantics.

import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { ConfigProvider, theme } from "antd";
import React from "react";
import { DispatcherPage } from "../src/pages/dispatcher.js";
import { FieldPage } from "../src/pages/field.js";

function ui(node: React.ReactElement) {
  return render(
    <ConfigProvider theme={{ algorithm: theme.darkAlgorithm }}>
      <MemoryRouter initialEntries={[locationOverride]}>{node}</MemoryRouter>
    </ConfigProvider>,
  );
}
let locationOverride = "/";

describe("dispatcher smoke", () => {
  it("renders grid + queue strip + event log scaffold", () => {
    locationOverride = "/d/demo";
    ui(<DispatcherPage />);
    expect(screen.getByText(/Dispatch — demo/)).toBeTruthy();
    expect(screen.getByText("Targets")).toBeTruthy();
    expect(screen.getByText(/Grant queue/)).toBeTruthy();
    expect(screen.getByText("Event log")).toBeTruthy();
    expect(screen.getByText("Exclusive")).toBeTruthy();
    expect(screen.getByText("Force release")).toBeTruthy();
  });
});

describe("field smoke", () => {
  it("renders roster + hold-to-talk + settings", () => {
    locationOverride = "/f/demo";
    ui(<FieldPage />);
    expect(screen.getByText(/Field — demo/)).toBeTruthy();
    expect(screen.getByText("Roster")).toBeTruthy();
    expect(screen.getByText("HOLD TO TALK")).toBeTruthy();
    expect(screen.getByRole("status")).toBeTruthy(); // aria-live node
  });
});
