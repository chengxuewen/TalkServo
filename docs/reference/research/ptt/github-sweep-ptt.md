# GitHub Sweep: PTT Keyword Family

> Purpose: exhaustive GitHub repository search across the push-to-talk keyword family, closing the "did you search GitHub exhaustively?" gap for `ptt-landscape.md`.
> Method: unauthenticated GitHub Search API (`sort=stars&per_page=30`), 7 queries, snapshot 2026-09-28. No rate-limit hits.
> Inclusion filter: stars >= 10 OR `pushed_at` on/after 2023-01-01. 161 unique repos returned across queries; 137 pass the filter and are all listed below.

## Verdict legend

| Verdict | Meaning for TalkServo |
|---------|------------------------|
| `peer` | Software voice-PTT / walkie-talkie / intercom **comms** system — benchmarkable against D1-D5 |
| `peer-adjacent` | Real-time voice comms, but full-duplex-only, hardware/embedded radio, or vendor intercom — context, not a direct software peer |
| `not-voice` | Keyword match without voice-comms (STT dictation, floor-plan/elevator false positives, scraped junk, text chat, AI assistants) |
| `infra` | Server / SDK / container / utility that enables PTT voice rather than being a PTT system |
| `stale-toy` | Demo, tutorial, or abandoned toy |
| `UNVERIFIED` | Description empty or ambiguous — voice-PTT status **cannot** be confirmed from metadata (OpenPTT-precedent skepticism applied; name match != voice peer) |

Flags: `KNOWN` = already in ptt-landscape known set; `ARCH` = archived on GitHub.

## Results (137 repos, deduped across queries, sorted by stars)

| Repo | ★ | Pushed | Lang | Description (EN one-liner) | Verdict | Flag |
|------|---|--------|------|----------------------------|---------|------|
| ExperienceLovelace/ha-floorplan | 1614 | 2026-09-13 | TypeScript | Home Assistant floor-plan control card | not-voice (floor-control false positive) | |
| peteonrails/voxtype | 1563 | 2026-09-27 | Rust | Voice-to-text push-to-talk for Wayland | not-voice (STT dictation) | |
| meshenger-app/meshenger-android | 963 | 2026-09-21 | Kotlin | P2P voice/video phone app for local networks | peer-adjacent (full-duplex, not PTT) | |
| atomic14/esp32-walkie-talkie | 650 | 2025-06-14 | C++ | ESP32 walkie-talkie via UDP broadcast / ESP-NOW | peer-adjacent (embedded HW) | |
| nicosandller/easy-floorplan | 595 | 2026-09-28 | TypeScript | Home Assistant floorplan card w/ editor | not-voice (floor-plan) | |
| talkkonnect/talkkonnect | 363 | 2026-09-02 | Go | Headless Mumble client as transceiver / walkie-talkie / intercom | peer | KNOWN |
| andreapianidev/WalkieTalkie | 350 | 2026-09-26 | Swift | Walkie-talkie app for iOS/macOS/Android | peer | |
| RealCorebb/bbTalkie | 335 | 2026-04-13 | C | Hands-free mini talkie gadget, VAD + speech-to-text display | peer-adjacent (embedded AI toy) | |
| yulrizka/osx-push-to-talk | 314 | 2024-07-26 | Swift | macOS menubar mic-mute-on-keypress (PTT gating for any voice app) | infra | |
| chrisneagu/FTC-Skystone-Dark-Angels-Romania-2020 | 314 | 2024-04-12 | Java | FTC robotics SDK fork | not-voice | |
| dchote/talkiepi | 303 | 2026-07-28 | Go | Headless Mumble client for RPi, GPIO push-to-talk button | peer | |
| sh123/codec2_talkie | 292 | 2026-02-24 | Java | Android amateur-radio Codec2/OPUS digital-voice transceiver | peer-adjacent (amateur radio HW) | |
| SaifAqqad/AHK_MicMute | 286 | 2026-01-03 | AutoHotkey | Windows mic control/mute utility | infra | |
| sh123/esp32_loraprs | 274 | 2026-04-01 | C++ | ESP32 LoRa APRS modem incl. Codec2 DV walkie-talkie feature | peer-adjacent (amateur radio HW) | |
| Sfedfcv/redesigned-pancake | 267 | 2021-05-16 | - | Scraped GitHub docs junk | not-voice | |
| ManojKumarPatnaik/Major-project-list | 254 | 2025-07-29 | - | Student project list | not-voice | |
| danderfer/Comp_Sci_Sem_2 | 210 | 2023-04-03 | Python | Course notes dump | not-voice | |
| per-simmons/murmur-youtube | 206 | 2026-08-22 | Swift | Push-to-talk dictation, native macOS/Windows | not-voice (STT dictation) | |
| deep-fingerprinting/df | 205 | 2023-03-25 | Python | CCS2018 acoustic fingerprinting paper code | not-voice | |
| raytheonbbn/hammer | 199 | 2021-05-04 | Java | Acoustic **data** modem sharing data over voice-only radio (ATAK) | not-voice (data-over-audio) | |
| primaprashant/awesome-voice-typing | 197 | 2026-09-28 | Python | Curated STT/voice-typing tools list | not-voice (awesome list) | |
| heardlabs/heard | 189 | 2026-09-19 | Python | Voice layer for coding agents | not-voice (AI agent voice) | |
| alexruperez/SpeechRecognizerButton | 184 | 2019-08-15 | Swift | UIButton PTT speech-recognition component | not-voice (STT component) | |
| js-labs/WalkieTalkie | 162 | 2025-03-09 | Java | Android WiFi walkie-talkie (js-collider framework demo) | stale-toy (demo) | |
| jettbrains/-L- | 160 | 2021-08-18 | - | Scraped W3C report | not-voice | |
| Xinyuan-LilyGO/T-TWR | 142 | 2025-08-05 | C | Programmable hardware walkie-talkie | peer-adjacent (HW radio) | |
| Experience-Monks/Invisible-Highway | 132 | 2018-04-26 | C# | AR floor-path control experiment | not-voice | |
| khlam/discord-sandboxed | 129 | 2023-02-04 | JavaScript | Alternative Discord client with privacy-focused push-to-talk | peer (Discord-based PTT voice) | |
| sh123/esp32_loradv | 122 | 2026-02-28 | C++ | ESP32 Codec2/OPUS UHF handheld transceiver | peer-adjacent (HW radio) | |
| openconcerto/MisterWhisper | 115 | 2025-12-03 | Java | Push-to-talk voice recognition using Whisper | not-voice (STT dictation) | |
| Rush/wayland-push-to-talk-fix | 114 | 2025-10-09 | C++ | Fix Discord PTT under Wayland | infra | |
| kangarooking/mobileclaw | 106 | 2026-04-06 | TypeScript | Multimodal voice+vision walkie-talkie for OpenClaw AI agents | peer-adjacent (AI-agent talkie) | |
| murtaza98/Walkie-Talkie | 105 | 2020-01-20 | Java | Android WiFi-Direct infrastructure-less comm app | UNVERIFIED (voice not stated; stale) | |
| akosma/bluewoki | 102 | 2016-02-06 | Objective-C | Bluetooth walkie-talkie for iOS | stale-toy | ARCH |
| gptguy/silentkeys | 96 | 2026-07-17 | Rust | On-device PTT dictation (Parakeet, Tauri) | not-voice (STT dictation) | |
| AutomationArt/LoraType | 92 | 2024-10-08 | HTML | LoRa urban teletype text tweets | not-voice (text radio) | |
| oddlama/whisper-overlay | 91 | 2024-07-26 | Rust | Wayland STT overlay via PTT hotkey | not-voice (STT dictation) | |
| hssstg/murmur | 90 | 2026-04-11 | C | Offline macOS voice-to-text, PTT in any app | not-voice (STT dictation) | |
| Mi-Walkie-Talkie-by-Darkhorse/Mi-Walkie-Talkie-Plus | 88 | 2023-05-29 | Smali | Enhanced Xiaomi Mi Walkie-Talkie radios companion | peer | |
| drajb/whisper-local | 83 | 2026-09-28 | Python | Offline AI dictation, PTT hotkey | not-voice (STT dictation) | |
| matiaspl/intercom | 82 | 2024-05-08 | JavaScript | DIY production intercom: Mumble server + RPi headless clients | peer | |
| britalmeida/push_to_talk | 80 | 2025-10-30 | Python | Blender Sequencer audio-recording add-on | not-voice | |
| solyarisoftware/WeBAD | 79 | 2022-07-15 | JavaScript | Browser audio-detection / speech-recording events API | not-voice (STT API experiment) | |
| RE3CON/1o11 | 77 | 2023-12-18 | C | Custom firmware for BaoFeng UV-K5/K6/UV-5R radios | peer-adjacent (analog/Digital radio HW) | |
| drewburchfield/macos-mic-keepwarm | 75 | 2026-08-31 | Swift | Fix PTT mic wake-up delay for transcription | not-voice (STT support tool) | |
| gms298/Android-Walkie-Talkie | 74 | 2017-05-03 | Java | Bluetooth PTT app for Android | stale-toy | |
| SydneyOwl/senhaix-freq-writer-enhanced | 74 | 2026-02-11 | C# | Frequency/channel programming software for Senhaix radio models | not-voice (radio programming tool) | ARCH |
| jaredrhod/backtalk | 73 | 2026-08-30 | Python | Hold-key talk-to-Claude-Code voice layer | not-voice (AI agent voice) | |
| AkuchiS/Yap | 72 | 2026-07-17 | Python | Offline hold-to-speak dictation at cursor | not-voice (STT dictation) | |
| shehackspurple/TTT-Pushing-Left | 71 | 2022-01-01 | - | Conference-talk slides repo | not-voice | |
| Eyevinn/intercom-frontend | 69 | 2026-09-25 | TypeScript | Low-latency web-based voice-over-IP intercom | peer | |
| x893/Codec2WalkieTalkie | 68 | 2017-06-10 | C++ | Codec2 walkie-talkie | peer-adjacent (radio, stale) | |
| dstd/micSwitch | 66 | 2022-11-09 | Objective-C | macOS mic mute w/ walkie-talkie-style single-key mode | infra | |
| rcspam/dictee | 65 | 2026-09-23 | Python | Linux PTT dictation, local Parakeet STT | not-voice (STT dictation) | |
| cyrinux/push2talk | 62 | 2025-10-12 | Rust | PTT integration for Wayland/X11/PulseAudio/PipeWire | UNVERIFIED (dictation vs comms unclear) | |
| nliaudat/floor-heating-controller | 61 | 2026-08-20 | - | ESPHome floor-heating firmware | not-voice | |
| lmacan1/talktype | 60 | 2026-03-16 | Python | PTT voice typing for terminal | not-voice (STT dictation) | |
| Unity-Technologies/GettingStartedWithBurst-Unite2019 | 59 | 2019-10-09 | C# | Unity Burst compiler talk companion | not-voice ("burst" false positive) | |
| Merve40/ptt | 58 | 2023-03-01 | JavaScript | Simple push-to-talk implementation | stale-toy (demo) | |
| zelloptt/zello-android-client-sdk | 58 | 2026-06-30 | Java | Legacy Android SDK of Zello Work PTT client | infra (commercial PTT SDK) | |
| arcasilesgroup/ai-engineering | 58 | 2026-09-28 | TypeScript | AI coding-agent governance ("control floor" wording) | not-voice | |
| fireship-io/vue-firebase-walkie-talkie | 53 | 2023-01-05 | Vue | Walkie-talkie-style chat app tutorial (Vue+Firebase) | stale-toy (tutorial) | |
| xuiltul/voice-input | 52 | 2026-02-18 | Python | PTT dictation + LLM refinement | not-voice (STT dictation) | |
| SmartWalkieOrg/VoicePing-Walkie-Talkie-AndroidSDK | 50 | 2026-07-17 | Java | VoicePing commercial walkie-talkie/PTT SDK for chat apps | infra (commercial PTT SDK) | |
| danInAustralia/WalkieTalkie | 48 | 2022-03-11 | Java | WiFi-Direct voice calls between Android devices | stale-toy | |
| lxe/yapyap | 47 | 2025-09-22 | Python | Fast simple PTT dictation | not-voice (STT dictation) | |
| devapro/LANwalkieTalkie | 44 | 2025-06-15 | Kotlin | Walkie-talkie / PTT app for local WiFi networks | peer | |
| ProjectSPAN/android-manet-ptt | 43 | 2017-08-09 | C | Android MANET push-to-talk voice chat | stale-toy (abandoned, was voice PTT) | |
| lxe/llm-companion | 43 | 2024-01-11 | JavaScript | Mobile PTT + TTS chat UI for OpenAI-like APIs | not-voice (AI chat) | |
| Aryia-Behroziuan/Other-sources | 43 | 2020-10-28 | - | Robotics survey references | not-voice | |
| mil-oss/walkitalkie | 43 | 2015-08-31 | Java | Android walkie-talkie + position app | stale-toy | ARCH |
| vohidjon123/google | 43 | 2022-04-13 | - | Scraped JS bundle | not-voice | |
| Dharshanaa-R/Emergency-Power-Cut-Detection-with-Automatic-Nearest-Floor-Rescue | 40 | 2026-03-31 | - | Elevator power-cut rescue system | not-voice (elevator) | |
| PyPtt/ptt_mcp_server | 39 | 2026-08-08 | Python | "PTT MCP server" (PTT here is likely the PTT BBS, not voice) | UNVERIFIED (likely not-voice) | |
| dmiddlecamp/walkie_talkies | 38 | 2016-01-24 | C++ | DIY hardware walkie-talkies guide | stale-toy | |
| ephendyy/sahabatfb | 38 | 2014-06-13 | - | Scraped Facebook userscript | not-voice | |
| ESPboy-edu/ESPboy_WalkieTalkie | 37 | 2023-01-03 | C | ESPboy handheld with SA868/SA818 radio module | peer-adjacent (HW radio) | |
| jonathanrandall/video_walkie_talkie_esp32 | 36 | 2025-05-10 | C++ | ESP32-cam video walkie-talkie | peer-adjacent (video, HW) | |
| BlockchainLabs/SpreadCoin | 33 | 2016-07-12 | - | Crypto mining whitepaper notes | not-voice | |
| notblackout/kf2-controlled-difficulty | 28 | 2018-04-12 | UnrealScript | Killing Floor 2 game mod ("floor") | not-voice | |
| zsith/launcher.user.js | 24 | 2015-07-26 | - | agar.io userscript junk | not-voice | |
| mnorlin/homecontrol | 21 | 2023-07-20 | JavaScript | Philips Hue control via floor map | not-voice | |
| Lifestylerr/DAWD | 20 | 2015-09-26 | - | Scraped HTML | not-voice | |
| deklaus/OpenValveControl | 18 | 2024-11-01 | HTML | Floor-heating valve controller | not-voice | |
| jeroenvdwaal/underfloor_heating_pump_controller | 16 | 2026-02-10 | - | Underfloor heating pump switch | not-voice | |
| drissi1990/googletagservices | 16 | 2019-08-17 | - | Scraped ad script | not-voice | |
| Rogerio111/Rogerio | 15 | 2015-08-27 | - | Scraped HTML game | not-voice | |
| edipurmail/scriptadsbygoogle.js | 14 | 2018-02-17 | - | Scraped ad script | not-voice | |
| liaoqingfu/video-voice-intercom | 13 | 2018-01-19 | C | SIP-based voice intercom | peer-adjacent (SIP intercom, stale) | |
| Ivan1931/Arduino-Elevator | 13 | 2019-05-07 | C++ | Model elevator controller | not-voice (elevator) | |
| renancunha/ae-quality-control | 12 | 2017-12-12 | Matlab | Ceramic-floor acoustic quality control | not-voice | |
| ryc3221775293/Six-elevators-ten-floors-group-control | 11 | 2018-12-25 | - | Elevator group-control logic (six cars, ten floors) | not-voice (elevator) | |
| Xpiatio/Hearthwave | 10 | 2026-09-22 | Python | GMRS family hub: phone/tablet client, Whisper transcription of transmissions | peer-adjacent (radio hub, half-duplex) | |
| yellowcrescent/atlas_control | 10 | 2014-02-19 | C | SCADA control framework | not-voice | |
| devapro/ptt-client-android | 9 | 2026-09-23 | Kotlin | PTT walkie-talkie client with self-hosted server | peer | |
| jalabulajunx/sidebar | 9 | 2026-08-07 | C | Private hardware voice intercom for kids, ESP32-S3, AES-256-GCM audio | peer-adjacent (embedded HW) | |
| Z1R343L-D77/Siemens-PLC-six-10-floor-elevator-control-program | 9 | 2026-03-29 | XSLT | Siemens PLC elevator control program (competition entry) | not-voice (elevator) | |
| keegan-carey/hubitat-floor-plan-dashboard | 9 | 2025-08-03 | TypeScript | Hubitat floor-plan dashboard | not-voice | |
| iamfatness/ZComms | 6 | 2026-09-06 | C++ | Zoom talkback/IFB station: hold key, voice lands in one panelist's ear | peer | |
| Hasan082/TalkBurst-Social | 5 | 2024-01-16 | Dart | "TalkBurst Social" conversation app | UNVERIFIED (voice unclear) | |
| david-spies/ptt-radio | 4 | 2026-09-27 | JavaScript | Browser walkie-talkie over P2P WebRTC: hold key to speak, zero audio servers | peer (direct WebRTC PTT) | |
| dx9674hnxw-spec/at2-bridge | 4 | 2026-09-14 | JavaScript | Self-hosted web/BLE control of Alervites-BaoFeng AT2 radio | peer-adjacent (radio control) | |
| Spelis/svc_intercom | 4 | 2026-03-12 | Java | Per-world voice/file broadcast plugin for Minecraft Simple Voice Chat | infra (game voice-chat plugin) | |
| Nothing-avil/Talk_Burst-ChatApp | 4 | 2024-02-18 | JavaScript | React text chat app | not-voice (text chat) | |
| devapro/ptt-server | 3 | 2026-09-17 | Kotlin | (no description; likely server of devapro/ptt-client-android) | UNVERIFIED | |
| SaintAngeLs/itml_voice_analysis_intercom | 3 | 2025-01-29 | Jupyter Notebook | CNN spectrogram classification ML study | not-voice (ML analysis) | |
| koliasa/Asterisk-PTT-Server | 2 | 2023-04-12 | Shell | Auto-install script for Asterisk PTT server on CentOS | infra | |
| kevinptt0323/ptt-ws-proxy | 2 | 2024-03-14 | JavaScript | Proxy server for "PTT" over WebSocket | UNVERIFIED (PTT may be the BBS, not voice) | |
| Starlordzz/sunsetripple | 2 | 2026-09-27 | Dart | Off-grid near-field Android voice intercom: WiFi-Direct full-duplex + Bluetooth PTT rooms, up to 6 devices, host auto-takeover | peer-adjacent (P2P HW-less talkie) | |
| haoqitianjue/ESP32-S3-Intercom- | 2 | 2025-11-26 | C | ESP32-S3 iLBC intercom, ESP-NOW + LoRa range, VAD, multi-device mixing | peer-adjacent (embedded HW) | |
| NapoII/My_gif_Icon_collection | 2 | 2024-06-08 | Python | GIF icon collection ("bursting" pun) | not-voice | |
| pvarki/docker-ptt-mumble_letsencrypt | 1 | 2025-05-16 | Smarty | Docker: Mumble server + Let's Encrypt | infra | |
| bbsmirror/openptt | 1 | 2023-03-19 | C | Mirror of OpenPtt/PttBBS (Taiwan BBS **software**) | not-voice (name-trap precedent confirmed) | |
| deepti-96/InterComm-Voice-Based-Email-Assistant | 1 | 2026-03-13 | Python | Voice-controlled email assistant | not-voice (AI assistant) | |
| fzrilsh/toice | 1 | 2026-08-15 | Dart | Offline P2P voice intercom for motorcycle touring convoys | peer-adjacent (offline P2P talkie) | |
| mcdonc/breakonthru | 1 | 2025-05-28 | C | Door-entry intercom hardware hack (unlock + voice) | peer-adjacent (door intercom HW) | |
| madhur24013/motolink-android-intercom | 1 | 2026-04-01 | JavaScript | RN Bluetooth helmet intercom, WebRTC voice rider-pillion | peer-adjacent (helmet intercom) | |
| 9982284/fbt_ai_voice | 1 | 2026-01-25 | C++ | ESP32 AI voice calling + multi-party intercom | peer-adjacent (embedded AI voice) | |
| lionel-arnaud/parental-broadcast | 1 | 2026-06-14 | C++ | ESP32 one-way voice paging to kids' room | peer-adjacent (one-way paging) | |
| JanWelker/bike_comm | 1 | 2026-06-13 | C | ESP32 mesh helmet voice intercom + BT HFP/A2DP | peer-adjacent (embedded HW) | |
| NEXhomeFujian/NEXCom | 1 | 2023-08-08 | Swift | SIP app for vendor NEXhome video door intercom | peer-adjacent (vendor door intercom) | |
| beepdt/Echo-B2B-Saas | 1 | 2025-12-29 | TypeScript | AI phone receptionist SaaS ("intercom" wording) | not-voice (AI receptionist) | |
| donapart/klatsch | 1 | 2026-03-18 | Python | Always-on local voice assistant hub | not-voice (AI assistant) | |
| GlomarGadaffi/tincan | 1 | 2026-09-01 | C++ | ESP32-S3 SIP voice endpoint: PTT G.711 over WiFi, P2P RTP | peer-adjacent (embedded SIP PTT) | |
| omerlevy1976/PTT-SERVER | 0 | 2026-09-25 | JavaScript | "PTT SERVER" (name-only) | UNVERIFIED | |
| venkataperugu1/ptt-token-server | 0 | 2026-05-14 | JavaScript | "PTT Token Server" (name-only) | UNVERIFIED | |
| ikanel/PolyPtt | 0 | 2025-06-20 | Python | "Polycom Ptt server" | UNVERIFIED | |
| drOngArt/ptt | 0 | 2026-04-23 | CSS | "PTT server to work with mobile application" | UNVERIFIED | |
| Jokerathome/ptt-server | 0 | 2026-03-01 | JavaScript | "PTT Walkie-Talkie Server" | infra | |
| harrowiersma/PTT | 0 | 2026-07-24 | Python | Self-hosted openPTT TRX server: GPS tracking, SOS alerts, ATIS, fleet dispatch | peer | |
| Cov4w/pttRadioServer | 0 | 2026-05-23 | C | (no description) | UNVERIFIED | |
| phgoodfriend/lan-voice-intercom6 | 0 | 2026-07-28 | JavaScript | (no description; apparent series of LAN voice intercom attempts) | UNVERIFIED | |
| phgoodfriend/lan-voice-intercom5 | 0 | 2026-07-28 | TypeScript | (no description; same series) | UNVERIFIED | |
| phgoodfriend/lan-voice-intercom4 | 0 | 2026-07-21 | TypeScript | (no description; same series) | UNVERIFIED | |
| bloodaroundfolk2/Talking-Ginger | 0 | 2026-09-28 | - | Game-download SEO spam | not-voice (spam) | |
| bloodharvest1920/Talking-Tom-Bubble-Shooter | 0 | 2026-09-28 | - | Game-download SEO spam ("burst" wording) | not-voice (spam) | |
| langrippen5/Talking-Tom-Bubble-Shooter-Full-Version | 0 | 2026-09-28 | - | Game-download SEO spam | not-voice (spam) | |

## Coverage claim — what each query returned

| Query (all with `sort=stars&per_page=30`) | API total | New voice-PTT **software** peers found |
|------|-----------|------------------------|
| q1 `push-to-talk` | 2376 | Overwhelmed by the 2025-26 STT-dictation wave (voxtype, silentkeys, dictee, ...); comms hits: khlam/discord-sandboxed, Merve40/ptt |
| q2 `push to talk in:name,description` | 2266 | Overlaps q1 heavily (~90%); adds zelloptt SDK, VoicePing SDK, devapro client, harrowiersma/PTT via q4-adjacency |
| q3 `walkie-talkie` | 1747 | Dominated by embedded/amateur-radio hardware + old demos; software peers: andreapianidev/WalkieTalkie, devapro/LANwalkieTalkie, Mi-Walkie-Talkie-Plus |
| q4 `ptt server` | 81 | Small pool; mostly zero-star UNVERIFIED servers + PTT-BBS name traps (bbsmirror/openptt, PyPtt, kevinptt0323) |
| q5 `intercom voice` | 89 | Mostly embedded/vendor/AI-assistant intercoms; web peers: Eyevinn/intercom-frontend, matiaspl/intercom |
| q6 `floor control` | 1028 | **Nothing new** — 100% false positives (floor plans, elevators, heating, SCADA, game mods). D1 "Floor" abstraction has no GitHub namesake |
| q7 `talk burst` | 12 | **Nothing new** — Unity Burst compiler, text-chat toys, SEO spam. "Talk burst" as a PTT term is essentially unused on GitHub |

Known-set visibility: only `talkkonnect/talkkonnect` (KNOWN) was returned. OpenPTT/OpenPTT, Theofilos-Chamalis/EVO-PTT, Gryt-chat/gryt, thepoison606/talktome, Radio-Link/Radio-link and RobCrack2023/Zello_Walk appeared in **none** of the 210 top-star results — this sweep cannot judge whether they are stale, only that they do not rank for these keywords on these terms.

The OpenPTT name-trap precedent recurs concretely: `bbsmirror/openptt` is PTT **BBS** software, not push-to-talk — confirming that every UNVERIFIED row above needs on-repo verification before entering ptt-landscape.

## Top 5 NEW peer candidates for ptt-landscape benchmark

1. **david-spies/ptt-radio** — browser hold-to-talk walkie-talkie over P2P WebRTC, "zero audio servers"; closest direct architectural contrast to D2 (signaling+SFU) — active (pushed 2026-09-27)
2. **harrowiersma/PTT** — self-hosted openPTT TRX server with fleet dispatch/GPS/SOS; server-side PTT feature checklist benchmark — active
3. **devapro PTT suite** (LANwalkieTalkie + ptt-client-android + ptt-server, Kotlin) — self-hosted client/server PTT pair; ptt-server UNVERIFIED but the pair matches D2 topology — all active 2025-26
4. **Eyevinn/intercom-frontend** — low-latency web VoIP intercom (TypeScript), closest web-client half-duplex comms peer — active
5. **matiaspl/intercom** — documented DIY Mumble-based intercom deployment (server + RPi headless clients); operational lessons for floor/PTT UX at 82-star scale

Honorable mentions (benchmark-worthy but classified infra/adjacent): zelloptt/zello-android-client-sdk and VoicePing SDK (commercial PTT SDK surfaces), SmartWalkieOrg, Xpiatio/Hearthwave (GMRS software hub), Starlordzz/sunsetripple (WiFi-Direct + BT PTT rooms with host auto-takeover — note overlap with D1 floor-takeover semantics).

## Sources

Snapshot date: 2026-09-28. Unauthenticated GitHub Search API, `sort=stars`, `per_page=30`, 7 requests, zero rate-limit retries:

- https://api.github.com/search/repositories?q=push-to-talk&sort=stars&per_page=30
- https://api.github.com/search/repositories?q=push+to+talk+in%3Aname%2Cdescription&sort=stars&per_page=30
- https://api.github.com/search/repositories?q=walkie-talkie&sort=stars&per_page=30
- https://api.github.com/search/repositories?q=ptt+server&sort=stars&per_page=30
- https://api.github.com/search/repositories?q=intercom+voice&sort=stars&per_page=30
- https://api.github.com/search/repositories?q=floor+control&sort=stars&per_page=30
- https://api.github.com/search/repositories?q=talk+burst&sort=stars&per_page=30

Known-peer list per `ptt-landscape.md`; verdicts derived solely from API `description`/metadata fields (no fabricated entries; CJK descriptions machine-translated to English by the sweep author).
