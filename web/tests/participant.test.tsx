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

// test-transmit (dev): the inline control flips the session seam, not just local state
// (PIT-28 family: a control whose job is an effect is tested on the effect).
import { setTestTransmit } from "../src/lib/session.js";
import { _setAudioContextCtorForTests } from "../src/lib/testAudio.js";
import { makeFakeAudioContextCtor } from "./fakeWebAudio.js";

describe("participant test-transmit control", () => {
  it("setTestTransmit routes through the module seam without throwing pre-session", () => {
    const { FakeAudioContext } = makeFakeAudioContextCtor();
    _setAudioContextCtorForTests(FakeAudioContext as unknown as typeof AudioContext);
    expect(() => setTestTransmit(true, "sine")).not.toThrow();
    setTestTransmit(false); // teardown path also must not throw
  });
});
