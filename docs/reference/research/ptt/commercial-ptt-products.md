# Commercial PTT Products — Benchmark Notes

> **Snapshot date: 2026-09-28.** All facts were captured live on this date via direct HTTP fetch
> of official product pages and Wikipedia (curl; search engines were rate-limited). Pricing, features,
> and page availability **drift quickly** — re-verify before citing in any published positioning.
> Cells that could not be verified from the fetched sources are marked **UNVERIFIED**.
> Audience: TalkServo whitepaper positioning ("floor control for PTT + full-duplex + hybrid", open-core Rust).
> This is a benchmark, not a sales pitch.

## 1. Feature matrix (verifiable products only)

| Product | Floor model | Channels vs groups | Priority / barge-in | Client platforms | Interconnect (radio/SIP/phone) | Pricing posture |
|---|---|---|---|---|---|---|
| **Zello / Zello Work** (Austin, TX; SaaS) | Walkie-talkie semantics: instant single-speaker voice "with one or many" over Wi-Fi/cellular [S1][S2] | Public user-created **channels** with moderators + private groups; "unlimited secure, private channels" [S1][S2] | Not documented on fetched pages (site markets "Emergency Alerts" — notification, not proven floor preemption) — **UNVERIFIED** | iOS, Android, Windows, Mac + earpieces/buttons/speaker-mic hardware accessories [S1] | **Radio gateways**: "LMR networks connect with Zello on smart devices, creating one seamless platform"; API/SDK access on paid tiers [S1] | Free consumer app; Zello Work Core $8.00/user/mo ($6.80 billed yearly), Plus $15.00 ($12.75 yearly), Enterprise adds **on-premise server option**; first responders free; education/nonprofit −10% [S1] |
| **Voxer** (Dallas, TX; SaaS) | No real-time floor arbitration: messages stream **live as recorded**, then replay as async voice messages (half-duplex walkie UX) [S3][S4] | 1:1 and group "walkie" chats; no public channel concept on fetched pages [S3][S4] | None documented — async-first model has no floor to preempt [S3][S4] | iPhone, Android, **Web** ("works on any network on iPhone, Android, and the Web") [S3] | None on fetched pages (Zapier integration for workflow automation instead) [S3] | Free tier + paid plans ("Choose Your Plan"); plan pages are JS-only, exact figures **UNVERIFIED**; enterprise/SMB tier marketed [S3][S4] |
| **Rave Mobile Safety** (acq. Motorola Solutions, Dec 2022) | Not a PTT floor: mass-notification + **panic-button / direct-to-911** workflows [S5] | Institution-scoped alerting (campus/agency); no talkgroups [S5] | n/a (alert escalation instead of floor control) [S5] | Mobile app (iOS/Android) per product line [S5] | 911-center integration (Rave 911 Suite, Smart911) — telephony/E911, not SIP radio [S5] | B2B SaaS sold to institutions (schools, agencies, corporate safety); per-customer, not self-serve public pricing — **UNVERIFIED** |
| **MCPTT — Unified Public Safety Network, PBC** (mcptt.com) | 3GPP Release 17-aligned MCPTT: "regionalized **floor control** with carrier-priority access ensures key-up times that meet or beat conventional LMR"; Android/iOS "share the same talkgroup state… the same millisecond of floor" [S6] | Geofenced **talkgroups**, per-unit location history, talkgroup live even during failover [S6] | **Yes**: carrier-priority access, emergency alerts, instant recall from dispatch console, multi-select patching [S6] | Rugged handheld/mobile radios + Android/iOS software clients + browser dispatch console [S6] | Deepest stack: unifies "LTE, **P25, DMR, analog, telephony**, and dispatch into a single operational fabric"; multi-carrier (FirstNet/Verizon/T-Mobile + 80 regional) [S6] | GSA-style; three deployments: Gov-Cloud SaaS (CJIS, 100% uptime SLA) / dedicated tenant in **your** AWS GovCloud-Azure Government / full **on-prem appliance** with cloud failover [S6] |
| **Hytera** (Shenzhen; hardware vendor, PoC ecosystem) | Device vendor, not a hosted floor service; PDC760 = "**DMR LTE hybrid** device… critical voice and broadband data services" (page marks it EOL) [S7][S8] | Managed via companion PoC/dispatch apps — not on fetched pages — **UNVERIFIED** | DMR standard tiered calls exist on the radio side — not on fetched pages — **UNVERIFIED** | Rugged radios/handsets (DMR, TETRA, LTE hybrid lines) [S7][S8] | Native LMR (DMR/TETRA repeater/trunked) + LTE hybrid devices; company acquired Rohde & Schwarz's TETRA PMR business [S8] | Hardware + dealer channel, no public per-seat pricing — **UNVERIFIED** |
| **Doro** (Lund, Sweden; hardware vendor, senior/care PoC ecosystem) | Not a floor service: eldercare phones/telecare maker whose "easy phones" + LTE devices pair with PTT-style care apps [S9] | n/a — care/telecare groups, not dispatch channels [S9] | No floor semantics documented — **UNVERIFIED** | Branded feature phones (own OS / KaiOS) + Android smartphones/watches [S9] | Cellular (4G/5G devices) + telecare backend; no radio interconnect [S9] | Hardware-led, subscription rides with care operators — **UNVERIFIED**; "EasyConnect" PTT brand page could not be located via official-site fetch/search — **UNVERIFIED** |

## 2. Brief items that failed live verification (do NOT cite as facts)

The research brief listed three additional products. As of 2026-09-28 **no official PTT product page
for any of them could be reached or confirmed**; search engines were rate-limited and the
Web-archive fallbacks produced no usable content. Treat these as unconfirmed names, likely
misattributions, and keep them out of the whitepaper until a primary source exists:

| Name in brief | What live fetch found | Status |
|---|---|---|
| "Novacap Mobile Connect" | `novacap.fr` does not resolve; `http://www.novacap.com/` = "Site Under Construction" placeholder; Wayback snapshots of `novacap.fr` (2022) and `mobileconnect.fr` (2025) rendered empty text | **UNVERIFIED** |
| "Atel Hybrid" (PTT) | `atel.fr` = temporary vehicle insurance; `atel.nl` = parked/for-sale domain; `atel.com` = ATEL Capital Group (leasing); `atel-usa.com` = LTE router/hotspot reseller (no PTT) | **UNVERIFIED** (same-name companies exist in unrelated sectors) |
| "Athelas" (PTT) | `athelas.com` = US healthcare-AI startup ("AI to grow your healthcare business"); `athelas.fi` does not resolve; no PTT entity found | **UNVERIFIED** |

Substitutions: the matrix keeps >=6 benchmark rows by including **MCPTT (mcptt.com)** — a live,
detailed MCPTT/dispatch offering that fills the "mission-critical interconnect" slot the brief
intended for Novacap — and **PocketTalker** ("Rave/PocketTalker-class") could not be listed in the
US App Store via the iTunes lookup/title-search APIs on the snapshot date, so Rave carries that
class alone [S10].

## 3. Product-level lessons — what makes a PTT product credible vs a toy

1. **One-press, sub-second key-up is the entry ticket.** Zello sells "push a button, talk instantly,
   audio plays in real time"; MCPTT measures itself against "conventional LMR key-up times". A PTT
   product without a stated floor-acquisition latency budget is a chat app with a badge.
2. **Floor arbitration is a spectrum, and pros need the top end.** Consumer products (Zello, Voxer)
   mostly lack documented preemption; MCPTT-class products advertise carrier-priority access,
   emergency barge-in, instant recall, and a shared "same millisecond of floor" state. Priority +
   audit are what dispatch buyers pay for.
3. **Channels/groups need an ownership hierarchy.** Zello's channel creator -> moderator model is
   why public channels scaled; Talkgroups need geofencing and admin control (MCPTT) in professional
   deployments. A flat room list is toy-grade.
4. **Browser-based dispatch console is a standard feature, not an add-on** (MCPTT: drag-and-drop
   talkgroup layouts, multi-select patching, tone alerts). A credible platform needs an operator
   surface distinct from talker clients.
5. **Platform parity across iOS/Android/Web/Desktop — and hardware accessories** (PTT earpieces,
   button mics) is assumed. Zello covers 4 OS families; MCPTT ships both apps and rugged radios.
6. **Interconnect beats rip-and-replace.** Both Zello and MCPTT lead with radio bridge stories
   (LMR gateways; P25/DMR/analog/telephony unification). Fleets are hybrid; a PTT engine without a
   gateway contract surface loses to one that has it.
7. **Compliance and deployment control are pricing levers.** On-prem / dedicated-tenant / gov-cloud
   options and FIPS/CJIS/E2EE claims (MCPTT, Zello Enterprise) are how vendors close public-sector
   deals that free SaaS tiers cannot touch.
8. **Voice is the wedge, data is the retention layer**: transcription, translation, AI summaries
   (Zello AI, Voxer AI), location tracking, audit logs, APIs/SDKs. And churn is real — once-solid
   free PTT apps (PocketTalker) can vanish from storefronts without notice.

## 4. Where TalkServo (self-hosted, open-core) can differentiate

- **Floor control as owned, inspectable state.** Every benchmark puts the floor engine behind a
  vendor SaaS (or a hardware dealer stack). An open-core Rust floor state machine (whitepaper D1)
  that teams can run, audit, and extend is the structural wedge — none of the closed vendors offer
  it at any price tier (Zello's on-prem Enterprise tier is still closed source).
- **On-prem by default, not premium SKU.** MCPTT sells self-hosted as its most expensive option;
  TalkServo's baseline deployment *is* self-hosted. Flip their upsell into our default.
- **Hybrid floor model no incumbent covers.** All six benchmarks implement exactly one UX (live
  floor or async voice). PTT + full-duplex conference + hybrid per-room (D1) has zero representation
  in the matrix — that is open positioning space, not a feature gap.
- **Gateway contract surface over proprietary gateways.** Interconnect exists at all benchmarks as
  vendor-controlled hardware/bridge SKUs. Exposing radio/PSTN interconnect as open interfaces on the
  SFU/signaling boundary (D2) invites third-party bridges instead of a single-vendor dependency.
- **Wire-format honesty.** The anti-pattern evidence in `docs/research/ptt-landscape.md` (Socket.io
  audio relay toys) is exactly what a self-built engine must not be; SRTP media path + FEC/jitter
  discipline (D3/D4) is the line between credible and toy, per lessons 1-2.
- **Do NOT compete on:** nationwide coverage, multi-carrier redundancy, or gov-program procurement
  machinery (UrgentPath, GSA schedule, carrier partnerships). That moat is telecom-scale; position
  the engine, not the network.

## Sources

All fetched live 2026-09-28 (curl; text extracted from HTML):

- [S1] Zello official site — `https://www.zello.com/` and `https://www.zello.com/pricing/` (plans, FAQ, radio-gateway and hardware copy)
- [S2] Wikipedia: Zello — `https://en.wikipedia.org/wiki/Zello` (channels/moderators, platforms, Zello Work vs free, emergency-response usage)
- [S3] Voxer official site — `https://www.voxer.com/` (live voice, platforms, AI summaries, Zapier, plan CTAs)
- [S4] Wikipedia: Voxer — `https://en.wikipedia.org/wiki/Voxer` (live-as-recorded semantics, async voice/text/photo/location, history)
- [S5] Wikipedia: Rave Mobile Safety — `https://en.wikipedia.org/wiki/Rave_Mobile_Safety` (product suite, Motorola Solutions acquisition 2022)
- [S6] MCPTT by Unified Public Safety Network, PBC — `https://www.mcptt.com/` (floor control/priority, deployment tiers, LMR interop, clients, console)
- [S7] Hytera EU: PDC760 product page — `https://www.hytera.com/eu/products/lte-radio/pdc760` (DMR+LTE hybrid, "critical voice and broadband data", EOL)
- [S8] Wikipedia: Hytera — `https://en.wikipedia.org/wiki/Hytera` (company profile, TETRA/DMR portfolio, Rohde & Schwarz PMR acquisition)
- [S9] Wikipedia: Doro (company) — `https://en.wikipedia.org/wiki/Doro_(company)` (senior/telecare devices, OS lines, Xplora acquisition)
- [S10] Apple iTunes lookup/search APIs — `https://itunes.apple.com/lookup?id=512850614`, `https://itunes.apple.com/search?term=pockettalker...` (PocketTalker: resultCount 0 on snapshot date)

Failed/unusable fetches recorded in section 2: `novacap.fr`, `novacap.com`, `mobileconnect.fr`
(+ Wayback), `atel.fr`, `atel.nl`, `atel.com`, `atel-usa.com`, `athelas.com`, `athelas.fi`,
`ravemobile.com` (JS-only shell), `doro.com` EasyConnect paths (404), `voxer.com/pricing` (404),
`hytera.com` PNC370 page guesses (404).
