// Participant page smoke — /room/:room shell renders offline-safe (no wire
// connection in jsdom): title, empty speaker grid, disabled toggle.

import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { ConfigProvider, theme } from "antd";
import React from "react";
import { ParticipantPage } from "../src/pages/participant.js";

function ui(el: React.ReactElement) {
  return render(
    <ConfigProvider theme={{ algorithm: theme.darkAlgorithm }}>
      <MemoryRouter initialEntries={["/room/demo"]}>{el}</MemoryRouter>
    </ConfigProvider>,
  );
}

describe("participant page", () => {
  it("renders room title + offline tag + disabled talk toggle", () => {
    ui(<ParticipantPage />);
    expect(screen.getByText(/Room — demo/)).toBeTruthy();
    expect(screen.getByText("offline")).toBeTruthy();
    expect(screen.getByText("nobody is talking")).toBeTruthy();
    expect(screen.getByText("WAITING FOR CONNECTION")).toBeTruthy();
    expect((screen.getByRole("button") as HTMLButtonElement).disabled).toBe(true);
  });
});
