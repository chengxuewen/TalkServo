# Standards Landscape: PTT Floor Control (OMA PoC, 3GPP MCPTT, Legacy Radio, China Private Net)

> Purpose: benchmark TalkServo's Floor model (docs/architecture.md §2, docs/whitepaper.md §5) against the formal standards world.
> Method: primary specs (3GPP TS) fetched and searched directly; secondary sources for radio standards; anything not verifiable at fetch time is marked **UNVERIFIED**.
> Date: 2026-09-28 | Status: research draft | Rule C1: English only.

## 1. OMA PoC (Push-to-talk over Cellular)

OMA standardized PoC in two releases; the architecture spec "OMA PoC — Push-to-talk over Cellular — Architecture, Candidate Version 2.0, 26 February 2008" is referenced in the public PTT literature ([Wikipedia: Push-to-talk](https://en.wikipedia.org/wiki/Push-to-talk), external-links section). OMA's current public spec index no longer lists PoC as an active work item, consistent with the ecosystem having migrated to 3GPP MCPTT/MCx ([OMA specifications page](https://www.oma.org/oma-standards/specifications/), fetched 2026-09-28; absence noted, wording of OMA's own status **UNVERIFIED**).

Claims about PoC internals:

- PoC reuses an OMA IMPS (Instant Messaging and Presence) XDM-style client/server model with PoC clients, a PoC server, and management lists (allowed/to-look-up). **UNVERIFIED** — the spec body was not fetchable on this network (openmobilealliance.org release PDFs unreachable; mirrors failed).
- The **Floor Control Function (FCF)** is PoC's arbitration role: it grants exactly one talk spurt per session, arbitrates concurrent requests (queue / reject / "last one to key wins" style policies), and revokes the floor on release or timeout. Signaling is carried as SIP INFO / event-package style messages between PoC client and server. **UNVERIFIED** — widely described in secondary literature but not confirmed against the spec text here.

Mapping to TalkServo: PoC's FCF is functionally our per-room centralized arbitrator (`arbitrate(&FloorState,&FloorRequest)` in `talkservo-core`, architecture.md §2.3). PoC's single-talk-spurt-per-session equals our `Exclusive` FloorMode. PoC never modeled an `Open` (conference) floor — that is where our abstraction is deliberately broader than the standard.

## 2. 3GPP MCPTT (Mission Critical PTT) — verified from spec text

Primary sources fetched 2026-09-28 and searched in full text:

- **TS 24.379** (Stage 3, on-network/off-network MCPTT signalling; Rel-18 version k10, `24379-k10.zip`) — [archive directory](https://www.3gpp.org/ftp/Specs/archive/24_series/24.379/).
- **TS 24.380** (MCPTT floor control procedures and media plane bearer — the actual floor protocol; Rel-18 k10) — [archive directory](https://www.3gpp.org/ftp/Specs/archive/24_series/24.380/).
- **TS 23.379** (Stage 2 architecture; latest k30) — [archive directory](https://www.3gpp.org/ftp/Specs/archive/23_series/23.379/).
- **TS 23.280** (common MCX architecture; latest k40) — [archive directory](https://www.3gpp.org/ftp/Specs/archive/23_series/23.280/).

Verified facts (clause wording located in fetched files):

1. Floor control is a *separate protocol layer* on top of call control: TS 24.379 states its procedures "refer to the floor-control procedures defined in 3GPP TS 24.380".
2. The floor arbitrator is the **floor control server** inside the controlling MCPTT function; peers have a **non-controlling MCPTT function** that mirrors floor state (`<floor-state>` = `floor-idle` | `floor-taken` observed by non-controlling functions, TS 24.379 §on floor status reporting).
3. Floor signalling is **XML MIME bodies carried over the media plane** of a pre-established session (`application/vnd.3gpp.mcptt-floor-request`), not on a control socket (TS 24.380; TS 23.280: "the media bearer carrying the floor control messages is always active").
4. Message vocabulary (TS 24.380, occurrence counts in k10 body): Floor **Request** (491), **Granted** (299), **Deny** (99), **Release** (355), **Taken** (341), **Idle** (248), **Queue** (203), plus **Revoke Request / Revoke** (authorized floor seizure) and **Floor Release Multi Talker**.
5. Floor participant client states (TS 24.380 state diagrams): `has no permission`, `pending Request`, `queued`, `has permission` / `permitted`, `pending Release`, `pending Floor Revoke` (+ session-level start/stop states). Server side has a `D: Floor Taken` state and a dedicated overriding-pre-emption procedure ("Receive Floor Request message with overriding pre-emptive floor priority").
6. **Priority is multi-dimensional by design.** The server decides via policy from: (a) the Floor Priority field in the request, (b) provisioned `<user-priority>` (TS 24.481), (c) `<num-levels-priority-hierarchy>`, (d) participant type, (e) call type. "Effective priority" is the server's computed decision — i.e. the spec itself distinguishes *static user ceiling* vs *per-request priority* vs *computed outcome* (matches our user/request priority split, architecture.md §2.3).
7. **Pre-emption has two flavors** (TS 24.380): "audio cut-in" (floor revoked from current talker) and "dual floor / overriding without revoke" — both talkers transmit simultaneously and distribution to listeners is configuration-driven. This is effectively a standardized *hybrid* floor moment; our `Hybrid` mode is the generalization.
8. **Call types**: MCPTT private calls (with and without floor control), **first-to-answer** calls, group calls, **emergency** group/private calls, **imminent-peril** calls, emergency alerts, broadcast/regroup. TS 24.379: emergency calls do not use emergency bearers — instead EPS bearer priority is adjusted; on-network private calls have mandatory floor control (24.379 §24.5 area).
9. Queueing is first-class: Floor Queue message carries queue position info; `Queue Info` and `Reject Cause` are defined fields of the floor signalling (TS 24.380 §8.2.3 field list: Floor Priority, Duration, Reject Cause, Queue Info, Granted Party's Identity).
10. Reliability model: per-message re-send counters + timers (e.g. T101 Floor Request, T20 Floor Granted default 1 s, C20 default 3, T7/C7 Floor Idle) — floor messages are repeated until acknowledged, unlike our current at-most-once WS events (see §5 gaps).

## 3. Legacy radio (TETRA / DMR / analog half-duplex)

- **TETRA** (ETSI standard, first version published 1995; TDMA, 4 channels per 25 kHz carrier; TMO networked mode and DMO direct-mode; group calling = "single button push connects the user to a selected call group and/or a dispatcher"; emergency buttons on terminals; terminals can also do full-duplex phone calls to PSTN) — [Wikipedia: TETRA](https://en.wikipedia.org/wiki/TETRA), fetched and searched. ETSI spec pages returned 403 on this network ([etsi.org](https://www.etsi.org/technologies/terrestrial-trunked-radio) — **UNVERIFIED** at spec level).
- **DMR** (ETSI; three tiers — I unlicensed, II conventional licensed, III trunked published 2012; TDMA) — [Wikipedia: Digital Mobile Radio](https://en.wikipedia.org/wiki/Digital_Mobile_Radio). Priority/emergency-call semantics of DMR Tier III: **UNVERIFIED** (spec not fetched).
- **Analog half-duplex / CB / aviation dispatch**: contention is resolved socially and by carrier dominance — "break, break" procedure words separate transmissions on a shared frequency ([Wikipedia: Push-to-talk](https://en.wikipedia.org/wiki/Push-to-talk)).

Arbitration UX implied by all three families: (1) press-to-talk = immediate-grant expectation when idle — grant latency, not protocol expressiveness, is what dispatchers feel; (2) **priority override is a user-visible physical action** (emergency button, dispatcher barge-in) — our `preempt: bool` request flag is the right primitive; (3) dispatcher/moderator roles are persistent authorities, distinct from transient per-request priority; (4) "who has the floor" must be broadcast to *all* listeners, not just the requester — radios announce by tone/LED; MCPTT answers with Floor Idle/Taken broadcasts.

## 4. China private-network PTT (B-Trunco / MMSCA)

B-Trunco is the brand of a Chinese LTE-based broadband trunking standard for public-safety and industrial private networks, developed under the MMOA (MiMOA) with MMSCA (China Radio Spectrum Association) involvement; it specifies multicast group-voice (PTT) over LTE. **UNVERIFIED** — no Wikipedia article exists ([attempted link](https://en.wikipedia.org/wiki/B-Trunco) returns no article), search engines blocked on this network, and no English official page was fetchable. Treat details (releases, PTT protocol stack, relation to 3GPP MCX) as unconfirmed until an official MMOA/MMSCA document is obtained. For PoC planning, the useful assumption is only: Chinese private-network customers interwork with B-Trunco-style systems, so our WS contract must stay close enough to MCPTT semantics (§2) to allow a future gateway (3GPP MCPTT itself is the reference these systems cite — also **UNVERIFIED** as a claim).

## 5. Standard concept → TalkServo mapping

| Standard concept (source) | TalkServo equivalent (architecture.md) | Verdict |
|---|---|---|
| OMA PoC FCF (UNVERIFIED) / MCPTT floor control server | Per-room centralized arbitrator in `talkservo-core` | aligned |
| Controlling vs non-controlling MCPTT function | Single server authority per room; no federation in PoC | skip for now |
| Floor Request / Granted / Deny / Release | `Request` / grant / `Denied` / `FloorRelease` WS messages | aligned |
| Floor Idle / Floor Taken broadcast to everyone | `FloorIdle` / `FloorTaken` events | aligned — keep broadcasting to all peers |
| Queued state + Floor Queue (queue position) | `FloorState.queue` + explicit `queued` event + position field | adopt |
| Revoke Request (authorized seizure of holder's floor) | `preempt: bool` merges requester-initiated + moderator-revoke | split later |
| Floor Priority field + `<user-priority>` ceiling + effective priority | `request_priority <= user_priority` ceiling, `arbitrate()` outcome | aligned |
| Audio cut-in vs dual-floor (overriding without revoke) | `Hybrid` mode semantics | adopt both as arbitration policies |
| Call types: private / group / first-to-answer / emergency / imminent-peril | Room kinds + `preempt` flag; no first-to-answer yet | adopt emergency tier |
| Media-plane floor signalling + re-send timers (T101/T20/C20) | Control-plane WS JSON, at-most-once events | **gap**: add ack/re-send or WS-level reliability |
| Floor-state `generation` | no standard equivalent found | our addition (race protection) — keep |
| Duration field (max talk spurts), Reject Cause enum | `floor_max_hold_ms` exists; deny reasons minimal | adopt reject-cause enum |

## 6. Adopt / skip for the PoC

**Adopt now (cheap, high standard-alignment):**
1. Message vocabulary parity: explicit `FloorQueued` event with queue position; typed `DenyReason` enum (mirroring Reject Cause).
2. Pre-emption split: `audio cut-in` (revoke holder) vs `dual floor` (both stream, listener-select) as two policy values on the preempt path — direct fit for Hybrid.
3. Emergency as a *call-type tier* that raises effective priority (not just a bool) — matches TS 24.379's emergency/imminent-peril distinction.
4. Broadcast floor outcomes (Idle/Taken/Granted-holder) to every room member, always — the radio UX lesson.

**Defer (add when real deployments demand):**
- Floor-message application-level re-send counters (WS reconnect semantics may already cover it) — revisit if field latency matters.
- Federation (controlling/non-controlling server split), off-network/DMO modes, MBMS broadcast, provisioning data models (TS 24.481/24.483/24.484), late entry, regroup, emergency alerts, MCPTT ID security.
- B-Trunco interworking — blocked on unverified sources; re-research with official MMOA docs.

**Skip for PoC:** media-plane floor signalling (our WS control channel is simpler and sufficient), SIP stack, XML MIME bodies, EPS/ARP bearer priority mapping (network-layer concern).

## 7. GitHub OSS corroboration (2026-09-28 sweep)

[github-sweep-ptt-standards.md](github-sweep-ptt-standards.md) confirmed: **zero** OSS OMA-PoC implementations, zero B-Trunco/MMSCA footprints (closed ecosystem), no credible OSS MCPTT core (only a 24-star Wireshark dissector and a hobby SIP-MCPTT shim); `wapipro` and `openmcptt` **do not exist on GitHub** — kill any citation of them. `mumble-voip/mumble` (8,303★, active) is the canonical live arbitration reference. Conclusion: no OSS floor-arbitration implementation exists to fork or interwork with — the D1 core is genuinely greenfield.

## Sources

Fetched and searched directly (2026-09-28):
- 3GPP TS 24.379 k10 — https://www.3gpp.org/ftp/Specs/archive/24_series/24.379/24379-k10.zip (directory: https://www.3gpp.org/ftp/Specs/archive/24_series/24.379/)
- 3GPP TS 24.380 k10 — https://www.3gpp.org/ftp/Specs/archive/24_series/24.380/24380-k10.zip (directory: https://www.3gpp.org/ftp/Specs/archive/24_series/24.380/)
- 3GPP TS 23.280 k40 — https://www.3gpp.org/ftp/Specs/archive/23_series/23.280/23280-k40.zip (directory: https://www.3gpp.org/ftp/Specs/archive/23_series/23.280/)
- 3GPP TS 23.379 archive directory — https://www.3gpp.org/ftp/Specs/archive/23_series/23.379/
- 3GPP TS 22.379 archive directory — https://www.3gpp.org/ftp/Specs/archive/22_series/22.379/
- Wikipedia: Push-to-talk — https://en.wikipedia.org/wiki/Push-to-talk (OMA PoC Architecture v2.0 2008-02-26 reference; analog dispatch "break" practice; PTToC)
- Wikipedia: TETRA — https://en.wikipedia.org/wiki/TETRA
- Wikipedia: Digital Mobile Radio — https://en.wikipedia.org/wiki/Digital_Mobile_Radio
- OMA specifications index — https://www.oma.org/oma-standards/specifications/ (PoC not listed as active)

Secondary/unverified (recorded for completeness):
- Wikipedia: Push-to-talk over Cellular — https://en.wikipedia.org/wiki/Push-to-talk_over_Cellular (no article; OMA PoC internals rely on UNVERIFIED secondary knowledge)
- Wikipedia: Mission critical Push-to-Talk / MCPTT — https://en.wikipedia.org/wiki/Mission_critical_Push-to-Talk (no article; MCPTT claims all rest on TS 24.379/24.380/23.280 text above)
- ETSI TETRA/DMR technology pages — https://www.etsi.org/technologies/terrestrial-trunked-radio (HTTP 403/404 on this network)
- B-Trunco — https://en.wikipedia.org/wiki/B-Trunco (no article); official MMOA/MMSCA English page not found on this network — section 4 is explicitly UNVERIFIED.
