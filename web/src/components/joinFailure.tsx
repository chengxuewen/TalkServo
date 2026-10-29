// JoinFailure (entry-flow wave, D29-#6=A): shared terminal-failure card for the three
// join pages. Tiers keyed off state the client ALREADY classifies (signal.ts):
//   expired  (auth-expired / 4010 family) — retry never succeeds, operator mints anew
//   removed  (4410 + the operator's reason, R7) — decision, not weather
//   neither  → render nothing (ordinary reconnect flow owns the UI)
// NO "apply to join" affordance: that is the lobby's territory (D29-#2=B, deferred).
import { Alert, Typography } from "antd";
import type { SessionStatus } from "../lib/session.js";

export function JoinFailure({ status }: { status: SessionStatus }): React.ReactElement | null {
  if (status.expired) {
    return (
      <Alert
        type="warning"
        showIcon
        message="This link has expired"
        description="Ask your operator for a fresh join link — retrying will not succeed."
        style={{ marginBottom: 12 }}
        data-testid="join-expired"
      />
    );
  }
  if (status.removedReason !== null) {
    return (
      <Alert
        type="error"
        showIcon
        message="You were removed from the room"
        description={
          <Typography.Text>
            {status.removedReason}
            {/* R7: surface the operator's words verbatim — no retry will follow */}
          </Typography.Text>
        }
        style={{ marginBottom: 12 }}
        data-testid="join-removed"
      />
    );
  }
  return null;
}
