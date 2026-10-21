// S7.5 (plan-7 D23 R3 + the 2026-10-20 walkthrough): the speaking surfaces must
// tell two different things apart —
//   standing:  the dispatcher took your speaking PRIVILEGE away (mirrored
//              `muted` roster, stays until un-muted);
//   transient: this one request was refused by the floor (FloorDenied).
// The wire deliberately shares ONE reason value (`unauthorized`) between the
// muted-request refusal (Rule 2) and the command-authorization refusal (Rule 10)
// because a new enum variant would leave older clients unable to parse a denial
// they have never seen (D16 compat). The roster is what disambiguates here.

/** Standing state label — the peer is muted by an authorized actor. */
export const MUTED_BY_DISPATCHER = "Muted by dispatcher";

/**
 * Terminal state (plan-9 R7-b): the console removed this peer from the room and the
 * transport will not retry, because the close code says the removal was deliberate.
 * Distinct from "disconnected" on purpose — the network did not fail, an operator
 * acted, and a peer who is told neither will keep pressing a dead button.
 */
export const REMOVED_BY_DISPATCHER = "Removed by dispatcher";

/** Human wording for a floor refusal. `selfMuted` picks the mute reading of
 *  `unauthorized` over the command reading (modules/01 Rule 2 vs Rule 10). */
export function denialCopy(reason: string, selfMuted: boolean): string {
  switch (reason) {
    case "unauthorized":
      return selfMuted
        ? `${MUTED_BY_DISPATCHER} — the floor refuses you`
        : "Refused — that command is not yours to send";
    case "busy":
      return "Floor busy — the queue is full";
    case "preempt_priority":
      return "Pre-empt refused — priority must be strictly higher";
    case "exceeds_ceiling":
      return "Refused — priority above your ceiling";
    case "rate_limited":
      return "Too fast — wait a moment, then press again";
    case "not_member":
      return "Refused — you are not a member of this room";
    case "no_media":
      return "Refused — no live media path for you yet";
    default:
      // an unknown reason must still be readable, never blank
      return `Refused (${reason})`;
  }
}
