# GitHub Saturation Sweep: PTT-Standard & Dispatch-Console Implementations

> Purpose: close the exhaustive-GitHub gap for the standards/interworking family behind [standards-ptt-mcptt.md](standards-ptt-mcptt.md) — do OMA-PoC / MCPTT / B-Trunco / Zello / dispatch-console implementations actually exist as OSS on GitHub, and what is mineable? Informs the Converged-stage SIP/RTP gateway boundary (`RelayBackend`, architecture.md §7) and D2 (centralized signaling + SFU).
> Method: unauthenticated GitHub Search API (`/search/repositories`, `sort=stars&per_page=30`), 16 search queries + 1 follow-up (`murmur language:rust`) + 9 direct `repos`/`readme`/`users` GETs. One rate-limit retry (btrunco first attempt, succeeded). Filter for the table: **stars >= 10 OR pushed_at >= 2023-01-01**; name-match noise clusters above that bar are aggregated, never dropped silently.
> Date: 2026-09-28 | Snapshot of all numbers at fetch time | Anything not confirmed against code/README is marked **UNVERIFIED**. Rule C1: English only.

## 1. Verdicts on the floating candidates (project discussion)

| Candidate (as floated) | GitHub status @2026-09-28 | Evidence |
|---|---|---|
| **wapipro** ("OMA PoC server") | **NOT FOUND — does not exist on GitHub** | Search `wapipro`: total_count=2; both irrelevant (`Selvianagy/WAPIProject`, 0* C#, no description; `matiBuet/WApiProxy`, 0* JS proxy, 2022). Treat the discussion reference as a mis-memory or off-GitHost location; do not cite it. **UNVERIFIED by construction.** |
| **openmcptt** ("MCPTT OSS") | **NOT FOUND — account itself 404** | `users/openmcptt` → 404; `repos/openmcptt/{openmcptt,core,docs}` → 404 x3; no `openmcptt` owner among the 40 `MCPTT` search hits. Zero GitHub footprint. |
| **mumble-voip/mumble** (server reference impl) | **CONFIRMED — canonical and very active** | 8303*, pushed 2026-09-26, C++, not archived, 1396 forks. Reference `murmur` server ships in-tree (`src/murmur/` confirmed via contents API). |
| **Go murmur port** | **DEAD** | `layeh/murmur` → 404 (repo deleted). Only companion `layeh/murmur-cli` (30*, Go) survives, **archived** 2023-06. |
| **Rust murmur port** | **NONE** | `murmur language:rust` (115 total): only MurmurHash libs + look-alike speech-to-text apps. No Mumble-server port exists. |
| **Any MCPTT server/core** | **NICHE/TOY ONLY** | No production-grade OSS MCPTT core; 3 credible-but-small artifacts (§2). The one non-trivial core (`cmvgg/SIP-MCPTT-Core`, C, hobby-grade) — conformance **UNVERIFIED**. |
| **Any OMA PoC server/client impl** | **ZERO** | `"oma poc"`: 10/10 matches are proof-of-concept/Omarchy noise ("oma" = the Omarchy package manager). `"poc server" push to talk`: total_count=0. Corroborates standards-ptt-mcptt.md §1 (PoC ecosystem migrated away, no OSS lineage to interwork with). |
| **B-Trunco / MMSCA OSS** | **ZERO** | `B-Trunco`: total_count=0. `btrunco`: 1 hit — unrelated Minecraft-style plugin (non-English description, omitted). Confirms standards-ptt-mcptt.md §4: closed Chinese ecosystem, nothing to mine on GitHub. |
| **Zello protocol (OSS)** | **ZERO** | `zello protocol`: total_count=0. Proprietary; no reverse-engineering repo footprint under that phrasing. |

## 2. Sweep table — qualified repos (stars >= 10 OR pushed >= 2023)

Judgment-relevant rows in full; pure name-collision clusters aggregated in §3. Categories per spec; radio-adjacent (DMR/P25) items marked in verdict text.

### 2.1 PTT platform / dispatch console

| Repo | * | Pushed | Lang | Description (API, trimmed) | Category | Relevance verdict |
|---|---|---|---|---|---|---|
| pushcommdigital-gif/pushcomm-community | 1 | 2026-09-01 | TypeScript | PushComm Community Edition — free, self-hosted push-to-talk over cellular platform... | dispatch-console | **Floor-semantics source + top watchlist.** README-verified: group channels / all-call / private 1:1, *half-duplex with a server-arbitrated floor* (LiveKit SFU), browser dispatch console, SOS/lone-worker with dispatcher acknowledge, recording+CDR, Docker self-host, AGPL-3.0. Production core of a commercial product — the closest OSS analog to TalkServo's shape. |
| mumble-voip/mumble | 8303 | 2026-09-26 | C++ | Open-source, low-latency, high quality voice chat software | mumble-server-port (upstream itself) | **Floor-semantics source** (precedent, not standards): priority-speaker = standardized pre-emption UX, channel ACL/Group = talkgroup permission model, in-tree `src/murmur`. Already profiled in oss-voice-infrastructure.md; this sweep confirms it is the *only living* reference server. |
| daryljones/radio-console | 0 | 2026-08-15 | C# | Cross-platform DVM dispatch console — Avalonia/.NET 10 port of DVMProject/dvmconsole with P25/DMR voice RX/TX. AGPLv3 | dispatch-console | Dispatcher-UX + P25/DMR workflow precedent. Caution: upstream `DVMProject/dvm` now 404 — the port is orphaned; mine layout/UX ideas only. |
| FreePBX/paging | 5 | 2026-07-17 | PHP | FreePBX Paging module — paging groups to make announcements | dispatch-console | One-to-many floor precedent: a page group is an `Open`-mode broadcast floor with a persistent dispatcher holder. |
| jawaadmerali/siren-dispatch | 0 | 2026-04-19 | TypeScript | Siren — unified 911 voice intake and AI dispatch console (Next.js + Express + MongoDB + Claude) | dispatch-console | Demo toy; irrelevant. (Trend signal: AI-assisted dispatch consoles shipping in 2026.) |
| areycruzer/kwik112 | 0 | 2026-09-12 | TypeScript | Kwik 112 — AI voice call-taker and dispatch console for India's 112 line | dispatch-console | Demo toy; irrelevant (same trend signal). |

### 2.2 MCPTT family

| Repo | * | Pushed | Lang | Description (API, trimmed) | Category | Relevance verdict |
|---|---|---|---|---|---|---|
| nemergent/MCPTT-Wireshark-Dissector | 24 | 2019-10-31 | Lua | Mission Critical Push To Talk (MCPTT) Wireshark dissector | MCPTT | **Interworking reference — most credible MCPTT artifact on GitHub.** Wire-level message/field vocabulary (floor request/granted/deny XML bodies per TS 24.380); use as an independent cross-check of `talkservo-core` WS enum coverage when the gateway stage comes. Stale (2019) but self-contained. |
| nandavelugoti/critical-access | 13 | 2020-04-10 | Java | Android application for Mission Critical Push to Talk (MCPTT) functionality | MCPTT | Study-grade client; 6 years stale. Irrelevant for PoC. |
| cmvgg/SIP-MCPTT-Core | 4 | 2025-03-21 | C | MCX-SIP-Comm — mission-critical communication system using SIP and MCPTT for secure Push-to-Talk... | MCPTT | **The only OSS "MCPTT core" skeleton found.** README-verified structure: SIP signaling via sofia-sip, TLS + SRTP, pthreads session management, Docker. Reads portfolio/student-grade; TS 24.379/24.380 conformance **UNVERIFIED**. Mine: naming/shape of a controlling-function core, not code. |
| usnistgov/psc-ns3-module | 4 | 2024-10-16 | C++ | ns-3 module with public safety communication related models | MCPTT | Interworking reference (research): emergency/priority bearer modeling for pre-emption benchmarks. |
| usnistgov/MCV-QOE-TVO | 1 | 2023-12-07 | Python | Mission critical voice quality of experience measurement calibration software... | MCPTT | Interworking reference: MC voice QoE methodology (grant-latency / interruption metrics). |
| richakumari200/MCPTT | 0 | 2023-11-28 | - | RTCP Floor control | MCPTT | Academic exercise (one-line description); **UNVERIFIED** substance. |
| kojh11/mcptt_core | 0 | 2025-03-05 | - | (no description) | MCPTT | Placeholder, no language reported — likely empty. **UNVERIFIED**, irrelevant. |
| ThomasJeffers/mcptt-client-1- | 0 | 2026-08-09 | Kotlin | the one the ai studio created, second one! | MCPTT | AI-generated toy. Irrelevant. |
| ThomasJeffers/fuzzy-parakeet | 0 | 2026-09-20 | Python | my-ai-generated-mcptt-server | MCPTT | AI-generated toy. Irrelevant. |
| tonylampada/mcptts | 3 | 2025-04-09 | Python | Mac TTS with Model Context Protocol for AI assistants | unrelated-name-match | Name collision (MCP + TTS). Irrelevant. |
| minimajzichioca/mcpttb | 0 | 2026-08-23 | - | (no description) | unrelated-name-match | Empty. Irrelevant. |

### 2.3 OMA PoC family — **zero qualified entries**

Neither `"oma poc"` nor `"poc server" push to talk` nor `wapipro` surfaced any OMA Push-to-talk-over-Cellular implementation. The family is OSS-extinct on GitHub; §1 of standards-ptt-mcptt.md (UNVERIFIED PoC internals) cannot be improved via GitHub — the specs remain the only source.

### 2.4 SIP stacks / intercom interworking (gateway-relevant per architecture.md §7 Converged)

| Repo | * | Pushed | Lang | Description (API, trimmed) | Category | Relevance verdict |
|---|---|---|---|---|---|---|
| restsend/rsipstack | 210 | 2026-09-28 | Rust | SIP Stack Rust library for building SIP applications | SIP-intercom | **Primary candidate for an in-process Rust SIP gateway** (created 2024-11, pushed the day of this snapshot). Preferred over any C-FFI path for D5 discipline. |
| Televiska/rsip | 108 | 2024-06-15 | Rust | SIP Rust library (generator & parser) | SIP-intercom | Message-layer reference; dormant since 2024 (+ rsip-dns, 11*, RFC 3263). |
| freeswitch/sofia-sip | 353 | 2026-08-03 | C | Sofia-SIP is an open-source SIP User-Agent library, compliant with IETF RFC3261 | SIP-intercom | Canonical C UA stack — now maintained under the FreeSWITCH org and still active. Protocol-behavior oracle / fallback gateway lib. |
| iuridiniz/sofia-sip-sys | 12 | 2021-08-14 | Rust | Rust bindings for sofia-sip | SIP-intercom | Stale (2021) FFI binding — evidence against the C-FFI route given rsipstack's activity. |
| staskobzar/sip_stacks_examples | 30 | 2018-01-23 | C | Examples of SIP register UA with sofia-sip, pjsip, libeXosip and libre | SIP-intercom | Compact cross-stack comparison (register flows) — useful when selecting a gateway lib. |
| fonoster/routr | 1709 | 2026-09-08 | TypeScript | The future of programmable SIP servers | SIP-intercom | Interworking reference: programmable SIP edge/routing patterns for gateway topology. |
| n-IA-hane/esphome-intercom | 254 | 2026-09-27 | Python | VoIP Stack for ESPHome and Home Assistant - local SIP phones, HA softphone/router, phonebook... | SIP-intercom | Embedded SIP voice endpoint stack — field-device pattern (hardware PTT button → SIP). |
| GlomarGadaffi/tincan | 1 | 2026-09-01 | C++ | ESP32-S3 SIP voice endpoint: push-to-talk G.711 over Wi-Fi, peer-to-peer RTP. registers to the pocke[t]... | SIP-intercom | **Literal PTT-over-SIP endpoint**: half-duplex talk spurt on a SIP/RTP leg — precedent for how a PTT floor maps onto SIP media sessions. |
| tylerransdell/onedoor | 32 | 2026-08-21 | JavaScript | Tactical-speed door comm and control — SIP audio + WebRTC video, fully local | SIP-intercom | Small-scale SIP<->WebRTC bridge precedent — direct prior art for the `RelayBackend` boundary. |
| PetrShtuka/CallWaveKit | 28 | 2026-09-25 | Objective-C | Incoming SIP calls on iOS with PJSIP, CallKit and PushKit. Supports SPM and CocoaPods | SIP-intercom | Mobile inbound-call wake pattern (PTT client backgrounding concern). |
| TECH7Fox/sipcore-hass-integration | 332 | 2026-09-14 | TypeScript | A SIP client inside home assistant! | SIP-intercom | Adjacent (embedded softphone in automation hub); shape reference only. |
| rosteleset/SmartYard-Server | 32 | 2026-09-25 | PHP | Open-source IP intercom & video surveillance software platform (RBT/Teledom) | SIP-intercom | Intercom product-shape reference (RU market; Android 39* / iOS 19* clients in family). |
| mahirgul/rsipclient | 2 | 2026-09-23 | Rust | A multi-account SIP client with a built-in modern Web Dashboard, REST API, and IVR Engine, written in [Rust] | SIP-intercom | Rust SIP application patterns; maturity **UNVERIFIED** (2*). |
| 0x4D44/rsiprtp | 3 | 2026-05-08 | Rust | (no description) | SIP-intercom? | Purpose unclear from metadata — **UNVERIFIED**, needs README pass before use. |

### 2.5 Mumble-server ports

| Repo | * | Pushed | Lang | Description (API, trimmed) | Category | Relevance verdict |
|---|---|---|---|---|---|---|
| layeh/murmur-cli | 30 | 2023-06-20 | Go | ARCHIVED — manage a grpc-enabled murmur server from the command-line | mumble-server-port | Tombstone of the Go murmur rewrite (`layeh/murmur` itself now 404). Confirms: murmur ports die; C++ upstream remains the only implementation. |

### 2.6 Radio talkgroup-adjacent (DMR/P25 amateur infrastructure — nearest "talk group" implementers)

| Repo | * | Pushed | Lang | Description (API, trimmed) | Category | Relevance verdict |
|---|---|---|---|---|---|---|
| audric/GeuReflector | 7 | 2026-07-16 | C++ | GeuReflector is an extended fork of SvxReflector. It turns a single-instance reflector into a federa[ted]... | dispatch-console (radio-adjacent) | Concept mining: federated multi-site talkgroup arbitration (who bridges which talkgroup, when) — a distributed-floor case study. |
| xtpclark/sdrtrunk_recording_parser | 4 | 2023-10-10 | PLpgSQL | Scripts to parse the sdrtrunk recording dir and move stuff around by date and talk group | dispatch-console (radio-adjacent) | Recording/CDR-by-talkgroup schema ideas — feeds OQ-3 (recording boundary). |
| ivomod/bm-tg-profiler | 2 | 2026-03-18 | JavaScript | Manage BrandMeister talk group profiles for your DMR hotspot via web GUI or CLI | dispatch-console (radio-adjacent) | Talkgroup membership/profile data-model reference. |
| cektor/Digi-Voice- | 1 | 2026-08-18 | HTML | A Highly Advanced Cross-Platform Application That Enables Communication in Talk Groups Using Digital [voice] | dispatch-console (radio-adjacent) | Thin/unsubstantiated — **UNVERIFIED**. |
| AnzeSinigoj/S5-DMR-Codeplug | 1 | 2025-03-17 | - | DMR & Analog radio codeplug for Slovenian HAM operators, including repeaters, simplex channels, talk[group] | dispatch-console (radio-adjacent) | Static config dump; irrelevant. |

### 2.7 Floor-state / misc singletons

| Repo | * | Pushed | Lang | Description (API, trimmed) | Category | Relevance verdict |
|---|---|---|---|---|---|---|
| RohanDoshi21/MurMur_GRPC-Chat | 5 | 2024-07-04 | Go | group chat application with invite-based group creation, real[]time... | unrelated-name-match | Name collision (not Mumble). Irrelevant. |
| MurmurationsNetwork/MurmurationsServices | 6 | 2026-09-25 | Go | Index, Library and other microservices that implement the Murmurations protocol | unrelated-name-match | Different "Murmurations" protocol. Irrelevant. |

## 3. Name-collision noise registry (above the filter bar, aggregated)

Every qualified-but-irrelevant match, clustered — none dropped silently.

| Cluster (query) | Qualified | Largest members | Note |
|---|---|---|---|
| Echo test servers (`echoserve`) | 17 | methane/echoserver 170*, yoshi-pi 67*, kalmhq 15*, cilium 14*, rajesh-1234567890 13*, ricoberger 11* | total_count 1185 = substring `echoserve*` matching echo-server tooling. The VoIP carrier "Echoserve" has **zero GitHub footprint**; `jaracogmbh/echoserve` (2*, 2026) is a REST-API mocking tool — another collision. |
| MurmurHash3 Go libs (`murmur language:go`) | 13 | spaolacci/murmur3 1017*, twmb/murmur3 354*, busser/murmur 128* (secrets CLI!), reusee/mmh3 52*, tildeleb/hashland 50*, huichen 39* | Canonical trap: "murmur" in Go means the hash function. |
| Rust speech-to-text look-alikes (`murmur language:rust`) | 8+ | Kieirra/murmure 1079*, stusmall/murmur3 78*, letsgetrusty/Murmur 36*, panda850819 20*, murmur-io 15*, webprodigies 10* | Audio-adjacent name collisions, none Mumble-related. |
| Remote-sensing RSIPAC competition forks (`rsip`) | ~10 | 2021rsipac_TOP4 50*, TOP5 44*, 2022rsipac 39*, RSIPAC_Track2 37*, ElementMo/RSIP 11*, RSIP-Vision plugin 11* | "RSIP" = Remote Sensing Image Processing + old RSIPnet, not SIP. |
| Proof-of-concept / Omarchy noise (`"oma poc"`) | 10 | jrmmhm/omarchy-pocket 11*, rest 0-1* | "oma" = Omarchy package manager; "poc" = proof-of-concept. 0 OMA-PoC hits. |
| Student "talk group" dumps (`"talk group"`) | ~15 | pythontalk_gatebot 4*, kpgtoolkit 2* (Kenwood HAM), mediafeed-talk-group 2*, rest 0* | Course-project names; zero PTT-standard implementers. |
| Home-automation doorbell tail (`"intercom" sip`) | 14 | aarnaud/smart-analog-intercom 4*, ddv2005/intercom 3*, OpenIPC/intercom 3*, ha-bticino-c100x 2*, ha-abb-welcome 2*, + 9 more 0-1* | True SIP-intercom family but doorbell-edge grade; only §2.4 members earned table placement. |
| sofia-sip mirrors/forks/bindings (`sofia-sip`) | 6 | BelledonneCommunications 30* (mirror), ppizarro/luasofia 30* (Lua, 2015), davehorton 20*, jart 17* (2010), unispeech 11* (UniMRCP), distro-packaging 0-1* | No independent value beyond upstream (§2.4). |
| `mission critical` non-voice | 3 | schubergphilis/mcvs-docker-action 2* (vuln scanner), mahmood726-cyber/mission-critical 0* (fixture), SiliconSage24 0* (SEO spam) | Query overlaps MCX vocabulary but not PTT. |
| Misc collisions | 4 | WillG24/EzSip 0* (empty HTML), ARADHYA-ezsippet 0*, RottenBread/BTRuncommand 0* (non-English desc), Selvianagy/WAPIProject 0* | Covers `ezsip`/`btrunco`/`wapipro` remains. |

## 4. Coverage claim

- **Queries with zero new candidates** (nothing above the noise floor): `"poc server" push to talk` (0 results), `B-Trunco` (0), `btrunco` (1 noise), `zello protocol` (0), `oma poc` (10/10 noise), `wapipro` (2 noise), `ezsip` (3 noise), `echoserve` (all noise), `"talk group"` (no PTT-standard hits; DMR-adjacent only), `murmur language:go` (one dead-port artifact), `murmur language:rust` (no ports), `rsip` (2 real finds, rest remote-sensing), `sofia-sip` (1 canonical + mirrors), `"dispatch console" voice` (1 real find + 2 AI toys), `MCPTT` (5 real, 6 toy/empty), `mission_critical` (overlap only).
- **Saturated verdict**: 17 search queries + 9 direct resource GETs; 153 unique filtered repos reviewed. The PTT-standards OSS family on GitHub is effectively **empty of server-grade implementations**: no OMA PoC impl, no production MCPTT core, no B-Trunco, no Zello, no live murmur port. Real discoveries live one layer over: dispatch-console products (PushComm), SIP plumbing (rsipstack, sofia-sip, routr), and PTT-over-SIP endpoints (tincan, esphome-intercom).
- **Declared gaps** (not queried, honest boundary): OMA IMPS/XDM term family; `MCX` bare term; CJK-market PTT terms; GitHub *code* search API (repository search only); GitLab/SourceForge presence (e.g., whether OpenMCPTT lived off-GitHub — out of scope per this sweep's GitHub-only constraint).

## 5. What to mine from each confirmed find

- **pushcomm-community** — read how its server arbitrates the half-duplex floor (queue, all-call, private 1:1, SOS pre-empt) and its open-core licensing split; closest OSS neighbor of the TalkServo product thesis.
- **mumble-voip/mumble (`src/murmur`)** — priority-speaker grant/revoke flow and channel ACL evaluation as the field-tested pre-emption/permission semantics; not standards-derived but battle-hardened.
- **MCPTT-Wireshark-Dissector** — use its field tables as a differential test when hardening `talkservo-core` floor messages toward TS 24.380 parity (Reject Cause, Queue Info, Floor Priority).
- **cmvgg/SIP-MCPTT-Core** — mine the component naming (MCX-SIP core, controlling function, TLS-signaling/SRTP-media split) for the future gateway module layout; do not trust its conformance.
- **usnistgov psc-ns3-module + MCV-QOE-TVO** — borrow emergency-priority simulation setup and MC-voice QoE measurement definitions for grant-latency benchmarks.
- **restsend/rsipstack** — evaluate as the pure-Rust SIP dependency when the Converged `RelayBackend` stage starts (activity: pushed same-day at snapshot); Televiska/rsip as the message-parser fallback.
- **freeswitch/sofia-sip** — reference behavior for SIP UA edge cases (RFC 3263/NAT), via its still-active upstream.
- **fonoster/routr** — programmable routing patterns for mapping talkgroup IDs onto SIP dial plans.
- **GlomarGadaffi/tincan + n-IA-hane/esphome-intercom** — how a physical push-to-talk button maps to SIP session + RTP spurt lifecycle (setup-on-demand vs pre-registered).
- **tylerransdell/onedoor** — smallest working SIP-audio + WebRTC-video bridge; pattern prior art for bridging our WebRTC leg to a SIP leg.
- **FreePBX/paging** — paging-group data model as a concrete `Open`-floor (broadcast) implementation.
- **audric/GeuReflector** — federation arbitration rules (which site wins a concurrent talkgroup) as a distributed-floor case study.
- **xtpclark/sdrtrunk_recording_parser + ivomod/bm-tg-profiler** — recording-by-talkgroup schema (OQ-3) and talkgroup profile data model.
- **daryljones/radio-console** — dispatcher console layout/interaction set (with orphan-upstream caution).

## 6. Net verdict for TalkServo

1. **No OSS interworking lineage exists to inherit** — OMA PoC and MCPTT servers were never built publicly at scale; our WS floor contract can stay MCPTT-*vocabulary*-aligned without any repo dependency (standards-ptt-mcptt.md §6 "Skip for PoC: SIP stack" stands).
2. **The only real standard-adjacent artifacts** are a Wireshark dissector and a hobby SIP-MCPTT core — wire vocabulary mining, not code mining.
3. **Gateway stage has a concrete dependency short-list now**: `rsipstack` (active Rust) > `rsip`/`rsip-dns` (dormant Rust) > `sofia-sip-sys` (stale FFI) > `sofia-sip` C upstream (oracle).
4. **PushComm Community Edition is the single most important repo surfaced**: a live, self-hosted, server-arbitrated-floor PTT platform — track it for floor-semantics evolution and open-core positioning against commercial-ptt-products.md.
5. **Mumble remains the sole living reference server**; Go/Rust murmur ports are dead/absent — reinforces D3 (webrtc-rs media engine) since no murmur fork shortcut exists.

## Sources (snapshot 2026-09-28, unauthenticated api.github.com)

Search queries (`/search/repositories`, sort=stars, per_page=30): `"oma poc"`, `"poc server" push to talk`, `MCPTT`, `"mission critical" push`, `B-Trunco`, `btrunco`, `echoserve`, `"talk group"`, `"dispatch console" voice`, `murmur language:go`, `ezsip`, `rsip`, `sofia-sip`, `"intercom" sip`, `zello protocol`, `wapipro`, follow-up `murmur language:rust`.
Direct GETs: `repos/mumble-voip/mumble`, `repos/mumble-voip/mumble/contents/src`, `repos/openmcptt/{openmcptt,core,docs}` (404), `users/openmcptt` (+ `/repos`), `repos/layeh/murmur` (404), `repos/DVMProject/dvm` (404), `repos/{pushcommdigital-gif/pushcomm-community,cmvgg/SIP-MCPTT-Core,daryljones/radio-console,restsend/rsipstack}/readme` (raw).
Cross-refs: standards-ptt-mcptt.md (UNVERIFIED OMA internals, TS 24.380 vocabulary), architecture.md §7 Converged/RelayBackend + D2/D3/D5, commercial-ptt-products.md, oss-voice-infrastructure.md, ptt-landscape.md.
Caveat: stars/pushed_at are point-in-time; `wapipro`, `openmcptt`, `layeh/murmur`, `DVMProject/dvm` verified **absent**, not merely unranked.
