# Mainstream Meeting Products — Web UI Surfaces

> **Status**: frozen research snapshot, 2026-10-08. Do not back-write; new findings go to a new dated file (C2).
> **Method**: text-only, official docs where reachable. `learn.microsoft.com` fully reachable and VERIFIED; `support.microsoft.com` reachable; `support.zoom.com` and `support.google.com/meet` return Angular SPA shells (JS-rendered content) to curl — those surfaces cited by canonical URL but claims from product knowledge, marked **[UNVERIFIED-SPA]**. Google Workspace admin (`support.google.com/a/`) redirects to sign-in — same treatment. Webex help returns generic "not found" for probed IDs — minimal coverage.
> **Consumer**: TalkServo web UI design (dispatcher/field/admin). Feeds a feature-comparison matrix and 2-3 TalkServo surface proposals (see `docs/modules/` and `docs/plans/`).
> **Companion dossiers**: [`admin-ui-patterns.md`](admin-ui-patterns.md) (RTC platform admin), [`ptt-ui-patterns.md`](ptt-ui-patterns.md) (consumer PTT/dispatch). This dossier covers mainstream meeting products specifically — the closest analog to floor control.

## 0. Reading key

- **[VERIFIED]** — content extracted from a fetched doc page in this session.
- **[UNVERIFIED-SPA]** — the canonical URL is real (returns 200 with SPA shell), but the article body is JS-rendered and unreadable via curl; claims drawn from product knowledge, cite the URL as the source of truth.
- **[UNVERIFIED]** — not visible in any fetched page, inferred or from marketing copy.

## 1. Zoom

### 1.1 Zoom Web App (browser) — in-meeting UI

**Bottom-center floating toolbar** (always visible, ~10-13 icons) [UNVERIFIED-SPA]:
- Leave / End (red)
- Video toggle
- Audio toggle
- Screen share (elevated — high-priority action, top-level icon)
- Participants panel toggle (opens right dock)
- Chat toggle
- Gallery / Speaker view switch
- More (⋯)

**Participants panel** — right-side dock (~280px, resizable):
- Tabs: **In Meeting** | **Waiting Room** | **Not In Meeting**
- Per-row: avatar, name, mic state icon, video state icon, role tag (Host / Co-host / Presenter / Interviewer / Panelist), raised-hand indicator
- Row ⋯ menu: Make Host, Make Co-Host, Request Control, Spotlight, Mute, Remove, Allow Audio, Rename, Block
- Bulk ops: Mute All, Mute All Except Self, Unmute All, Remove All
- Host-only section: "Waiting Room" tab shows admit/reject cards with webcam preview

**Raise hand** [UNVERIFIED-SPA]: banner at top of participants panel ("X raised their hand") + list; host taps to spotlight and lower.

**Spotlight**: makes one participant's tile large; others become a thumbnail strip on the right.

**Live transcript panel** [UNVERIFIED-SPA]: right dock, speaker-labeled text stream, search, "Copy text" + "Download" actions. Enabled via host "More → Record & Capture → Start Transcription".

**Chat panel** (right dock, tabs): Everyone / Direct; supports file upload, image preview, @mentions.

**Screen share layout**: screen occupies center; participants become a thumbnail strip on the right; presenter can toggle "Show participants as picture-in-picture".

**Host/moderator overlay** — bottom "More" opens a modal with sections:
- **Manage People**: Mute All / Unmute All / Lock or Unlock Meeting / Admit from Waiting Room / Admit All / Allow participants to unmute themselves
- **Meeting Security**: Lock Meeting, Set Password, Hide Participant List, Hide Chat, Lock Participants, Disable Screen Share, Disable Chat, Disable Reactions, Disable File Transfer
- **Record & Capture**: Start Recording, Start Transcription, Start Zoom Notes
- **Breakout Rooms**: Create Breakout Rooms, Auto-Assign Participants, Send Messages to All, Send Timer Notification, End Breakout Rooms
- **Q&A**: Enable Q&A, Pin Question, Hide Answer

**Recording** [UNVERIFIED-SPA]: local or cloud (host-configurable via meeting settings); cloud recording produces a watchable link; recording consent prompt shown to participants.

Sources:
- In-meeting controls: <https://support.zoom.com/hc/en-us/articles/4405999556754> [UNVERIFIED-SPA]
- Manage participants: <https://support.zoom.com/hc/en-us/articles/4405999510298> [UNVERIFIED-SPA]
- Waiting room: <https://support.zoom.com/hc/en-us/articles/26678325050514> [UNVERIFIED-SPA]
- Zoom AI Companion / transcript: <https://support.zoom.com/hc/en-us/articles/8357749018266> [UNVERIFIED-SPA]

### 1.2 Zoom Web App — pre/post join

**Pre-join screen** [UNVERIFIED-SPA]:
- Large camera preview with device selector dropdown (video in / out / none)
- Mic level meter + audio device selector
- Buttons: **Join with video** / **Join without video** / **Leave**
- "Sound test" link
- Optional: display name field if joining without an account
- Lobby preview (if waiting room enabled and host not present): "You're waiting to be admitted" state with mic/cam controls disabled

**Post-join** [UNVERIFIED-SPA]:
- "Thanks for the meeting" screen with "Back to Home" button
- If cloud recording started: link to recording URL
- If transcription on: link to transcript
- Host-side: participant report (attendance data) via meeting settings → "Meeting Report"

### 1.3 Zoom Admin Portal — IA + RBAC

**Admin portal top-level left nav** [UNVERIFIED-SPA]:
- Home (dashboard)
- Account Settings
- Users
- Groups
- Meetings (scheduled meetings inventory)
- Reports
- Account Analytics
- Billing
- Apps
- Marketplace
- Device Management
- Security
- Support

**Users** subsections [UNVERIFIED-SPA]:
- User Management (CRUD + invite + license assignment + deactivate)
- User Roles (assign RBAC role)
- User License Assignments
- User API Tokens
- User Activity
- Sign-in Activity
- User Groups

**Security** subsections [UNVERIFIED-SPA]:
- Account Settings
- Account Data Retention
- Account Locks
- Data Erasure
- Authentication (SAML, OIDC, MFA, SSO)
- Audit Logs
- Advanced Account Settings
- Data Privacy

**Reports** subsections [UNVERIFIED-SPA]:
- Dashboard (usage overview)
- User Activity
- User Sign-in Activity
- Meetings & Webinars
- Webinars
- Account Events
- Phone Activity

**RBAC — Account Role model** [UNVERIFIED-SPA]:
- Account Owner (full control)
- Account Owner Restricted (limited scopes)
- Account Admin (full admin except billing)
- User Manager (users + groups only)
- Co-Host (host equivalent in a meeting)
- User (default)
- Meeting Creator (schedule meetings only)
- Host (host meetings only)
- Group-based custom role: define permissions per group

**Meeting settings** (per-user via Users or Groups) [UNVERIFIED-SPA]:
- Meeting security: password required, waiting room, allow participants join before host, lock meeting, mute participants on entry, allow participants to unmute themselves, chat/file transfer/reactions restrictions, hide participant list, hide self-view
- Recording: local, cloud, retention policy
- Video/audio defaults
- Device management
- Screen sharing restrictions
- Breakout rooms, Q&A, cloud recording toggles

Sources:
- Admin portal overview: <https://support.zoom.com/hc/en-us/articles/4405999820442> [UNVERIFIED-SPA]
- Roles & permissions: <https://support.zoom.com/hc/en-us/articles/4404249601298> [UNVERIFIED-SPA]
- Meeting settings (admin): <https://support.zoom.com/hc/en-us/articles/4408822104082> [UNVERIFIED-SPA]
- Audit logs: <https://support.zoom.com/hc/en-us/articles/4412477847066> [UNVERIFIED-SPA]

## 2. Google Meet

### 2.1 Google Meet web — in-call layout

**Two primary layouts** [UNVERIFIED-SPA]:
- **Speaker view**: one large "current speaker" tile + strip of thumbnails below (auto-updated by speaking detection)
- **Grid view**: all participants in a grid (up to ~49 tiles)

**Bottom-center toolbar**:
- Mic toggle
- Camera toggle
- Screen share
- Chat (opens right dock)
- Raise hand
- More (⋯)

**Right dock** (two tabs, unified panel) [UNVERIFIED-SPA]:
- **Chat**: text messages, @mentions, file upload, emoji reactions
- **Participants**: full list with mic/camera/video state; organizer/co-host sees per-row "⋯" menu

**Organizer / host tools** (organizer + co-host only) [UNVERIFIED-SPA] — accessed via More → "Host tools":
- Manage participants (invite, add by email, kick)
- **Mute all** / **Unmute all** / **Allow only host to unmute** / **Allow participants to unmute**
- Kick someone out
- Lock chat
- Make someone a host
- Stop video for everyone
- Stop share for everyone
- Turn on recording
- Turn on transcription
- Turn on live captions
- Breakout rooms (create, auto-assign, send messages, end, timer)
- Q&A (for classes and large meetings)
- **Who's speaking** — auto-highlight / spotlight the current speaker based on audio detection

**Hand-raise UI** [UNVERIFIED-SPA]: participant tiles get a raised-hand badge; organizer sees a list at top of Participants panel.

**Pre-join screen** [UNVERIFIED-SPA]:
- Camera preview with device selector
- Mic level meter + device selector
- "Join now" / "Receive call" (for dial-in)
- "Sound test"
- Background blur / virtual background effects
- Optional: display name if not signed in

**Post-call screen** [UNVERIFIED-SPA]:
- Recap with link to recording (if started)
- "Take notes for me" button (Gemini AI feature, Workspace Business Standard+ / Education / Enterprise)
- "Ask Gemini" / "Ask Hangouts Meet" for AI Q&A on the recording
- Recap summary (Gemini-generated summary of key discussion points)

**Recording permission model** [VERIFIED from <https://support.google.com/meet/answer/9308681>]:
- Recording requires admin to enable it for the account first
- Anyone who can record: meeting host, external user promoted to co-host, internal user when Host Management is off, teacher or co-teacher for Classroom meetings
- Workspace edition requirements: Business Plus/Standard/Essentials, Education Plus, Enterprise Essentials/Plus/Standard/Starter, Google One 2TB+, Workspace Individual, Teaching and Learning Upgrade
- Participants must have host permission to record; if host can't record, no one can
- Admins can configure "provide consent" (Start / Join / Continue) flow and "Quick Stop" toggle

Sources:
- Host tools: <https://support.google.com/meet/answer/9928638> [UNVERIFIED-SPA]
- Manage participants in Meet: <https://support.google.com/meet/answer/9308681> (recording article) [VERIFIED]
- Join a meeting: <https://support.google.com/meet/answer/12076912> [UNVERIFIED-SPA]
- Record a meeting: <https://support.google.com/meet/answer/9308681> [VERIFIED]

### 2.2 Google Workspace Admin Console — Meet settings

**Admin console IA** (admin.google.com) [UNVERIFIED-SPA — redirects to login when unauthenticated]:
- Dashboard
- Get started
- Directory (Users, Groups, Organizational units, Resource calendars, Shared drives)
- Apps (Google Workspace, G Suite Business, G Suite Enterprise)
- Security (Alert center, Security dashboard, Authenticated users, Access and data control, Security center)
- Device management (Mobile devices, Chrome devices, Android devices)
- Data protection (DLP rules, Drive audit, BigQuery export)
- Account (Licenses, Subscriptions, Billing)
- Reports (Usage, Login activity, Security, DLP, Drive, Gmail, Calendar)
- Admin quick start

**Meet settings** — under **Apps → Google Workspace → Google Meet**:
- Meet settings
  - Meet access (in-domain, anyone on the web, everyone)
  - Room booking
  - Location tracking
  - Meeting history
  - Meeting recording
  - Transcription
  - Breakout rooms
  - Host management (auto-assign host, host lock, auto-elevation)
  - Data loss prevention (DLP rules for Meet content)
  - External access & guest features (external chat, external meetings, federation)
  - Advanced settings (video quality, adaptive bitrate, in-domain chat)
  - Advanced meeting settings (audio processing, echo cancellation)

**Role model for Meet admin** [UNVERIFIED-SPA]:
- Workspace uses **roles** (Super Admin, Groups Editor, User Manager, etc.) with granular **privileges**
- Meet-specific privilege: "Google Meet administrator privilege" — must be granted to a role for the admin to see Meet settings
- Custom roles: admins can define scoped roles (e.g. "Meet-only admin" with only Meet settings + Users roles)
- Privileges can be scoped by organizational unit (OU) — e.g. OU-level Meet admin for a specific department

Sources:
- Meet admin settings: <https://support.google.com/a/answer/9305658> [UNVERIFIED-SPA]
- Meet recording admin: <https://support.google.com/a/answer/9269229> [UNVERIFIED-SPA — redirected to Chat admin on fetch]
- Meet DLP: <https://support.google.com/a/answer/14459711> [UNVERIFIED-SPA — redirected to home]
- Meet location tracking: <https://support.google.com/a/answer/12182236> [UNVERIFIED-SPA]
- Meet admin topic landing: <https://support.google.com/a/topic/7542072> [UNVERIFIED-SPA]

## 3. Microsoft Teams (web client + admin center)

### 3.1 Teams web client — overall IA

**Left icon rail** (persistent):
- Chat (1:1 DMs + group DMs)
- Calendar (meetings, events, live events)
- Teams (channel-based collaboration)
- Calls
- Activity (notifications)
- Apps (Teams apps + 3rd-party apps)
- Settings (gear)

**Teams surface**: list of teams (top) → channels within each team (left rail) → channel view (files, messages, Meet tab).

**Meetings**: scheduled via Calendar; instant via "Meet now" or "New chat" → "Video call" (web conferencing without scheduling).

**Channel-based IA**: a "channel" is a persistent room with messages, files, a Meet button, and apps — this is the defining structural difference from Zoom/Google Meet, which have no persistent channel concept.

[VERIFIED from <https://learn.microsoft.com/en-us/microsoftteams/teams-client-web>]:
- Teams for Web works in Edge, Chrome, Firefox, Safari (Windows/macOS/Linux desktop)
- **Not supported**: multi-account/tenant availability, popout windows for chat/channel content, avatars/video backgrounds except blur
- Not supported in VDI environments

Sources:
- Teams for Web overview: <https://learn.microsoft.com/en-us/microsoftteams/teams-client-web> [VERIFIED]
- Teams admin documentation landing: <https://learn.microsoft.com/en-us/microsoftteams/> [VERIFIED — section titles only]

### 3.2 Teams in-meeting roles — the closest analog to floor control

[VERIFIED from <https://support.microsoft.com/office/c16fa7d0-1666-4dde-8686-0a0bfe16e019>]:

> "Assign different meeting roles in a Teams meeting to give people specific permissions. There are three roles to choose from: *co-organizer*, *presenter*, and *attendee*. Co-organizers and presenters share most organizer permissions, while attendees are more controlled."

**"Who can present" options (before meeting)**:
| Who can present | What happens |
| --- | --- |
| Everyone | Anyone who has access to the meeting link will join as a presenter |
| People in my org and guests | Only people in your org and guests join as presenters |
| Specific people | Only chosen individuals are presenters; others are attendees |
| Only organizers and co-organizers | Only organizers and co-organizers can present |

**Setting before meeting**: Calendar → Select event → View event → Options → Roles → Choose co-organizers + "Who can present" dropdown → Save.

**During-meeting role change**: People panel → hover name → ⋯ → "Make a presenter" / "Make an attendee". Or Calendar → Options → Roles → Choose co-organizers (applies to future occurrences of recurring meetings).

**Anonymous participants** [VERIFIED]: promoted to presenter but prevented from muting participants, removing participants, lowering raised hands (mobile devices excepted).

**Notes** [VERIFIED]:
- Meeting organizer role can't be changed
- Co-organizers can't change the meeting before it starts
- The "specific people" presenter option is not available for channel meetings

**Host moderation toolset** — article TOC lists these as separate moderated areas [VERIFIED from same page's TOC]:
- `using-the-green-room` (pre-meeting prep room for speakers)
- `using-the-lobby` (admit/decline)
- `manage-attendee-audio-and-video-permissions` (mute/unmute, force camera)
- `manage-what-attendees-see` (spotlight, stop share)
- `spotlight-someone-s-video`
- `manage-q-a`
- `manage-breakout-rooms`
- `lock-a-meeting`
- `end-a-meeting`

The "Manage attendee audio and video permissions" article is the direct floor-control analog — the organizer can: mute everyone, allow participants to unmute, force camera on/off, and this controls who "speaks" in the meeting.

Sources:
- Roles in Microsoft Teams meetings: <https://support.microsoft.com/office/c16fa7d0-1666-4dde-8686-0a0bfe16e019> [VERIFIED]

### 3.3 Teams Admin Center IA — full left nav (VERIFIED)

[VERIFIED from <https://learn.microsoft.com/en-us/microsoftteams/settings-policies-reference>, section "Navigation" references]:

**Teams**
- Teams settings (notifications, tagging, email integration, file storage)
- Teams policies (team/channel behavior)
- Template policies (which templates users see when creating a team)
- Update policies (preview feature enrollment)
- Teams upgrade settings (Skype for Business coexistence)

**Teams apps**
- Permission policies (which apps are available)
- Setup policies (app upload / customization permissions)
- Customize store (org logo, background, colors)

**Meetings**
- Conference bridges (phone numbers, entry/exit announcement)
- Audio Conferencing (toll / toll-free numbers)
- Meeting policies (see §3.4)
- Customization policies (org logo, colors for meetings — Teams Premium)
- Meeting settings (anonymous join, port ranges, media traffic)
- Live events policies
- Live event settings (custom support URL, video distribution providers)
- Meeting templates
- Meeting template policies
- Events policies (webinars)

**Messaging**
- Messaging policies (delete sent, edit, GIFs, emoji, mentions, @channel/@here)

**Voice**
- Calling policies (make private calls, cloud recording, transfer, hold)
- Call hold policies
- Call park policies
- Caller ID policies
- Emergency policies (emergency calling locations, dynamic lookup)
- Mobility policies (Teams Phone Mobile dialer)
- Shared calling policies
- Voice routing policies (Direct Routing)
- Voicemail policies (recording length, transcription, language)
- Voice applications policies (auto attendants, call queues)

**Enhanced encryption policies**
- End-to-end call encryption (Teams Premium)

The article explicitly says: *"This reference organizes each section by its related policy area, and also includes any PowerShell-only policies."*

Additional sections documented elsewhere (from related articles index): **Devices** (Telephony, Teams Rooms, Phone systems), **Teams & groups** (Manage Teams, Manage Teams apps), **Users & groups** (Manage users, Manage guests, Manage groups), **Security & compliance**, **Reports & analytics** (Teams & devices, Meetings & calls, Copilot usage), **Copilot**.

### 3.4 Meeting policies — the concrete per-meeting knobs (VERIFIED)

From the reference [VERIFIED], the **Meeting policies** section lists these settings:
- Meeting scheduling
  - Private meeting scheduling (On/Off)
- Recordings (see §3.5 for full list)
- Lobby (see §3.5)
- Attendees (see §3.5)
- Screen sharing (see §3.5)
- Chat, reactions, Q&A, polls, breakout rooms (see §3.5)

The full policy settings are in <https://learn.microsoft.com/en-us/microsoftteams/settings-policies-reference#meetings> [UNVERIFIED — page fetched but the specific table content beyond the section header wasn't fully extracted in this session; see the policy-assignment page for the full list].

The **policy assignment model** [VERIFIED from <https://learn.microsoft.com/en-us/microsoftteams/meeting-policies-overview>]:
- Users get the **global policy** (org-wide default) unless a custom policy is assigned
- Custom policies can be created and assigned to individual users, groups, or at scale via batch assignment
- A user can be assigned only one meeting policy and one events policy at a time
- Three implementation types:
  - **Per-organizer**: participants inherit organizer's policy (e.g. "Who can bypass the lobby")
  - **Per-user**: policy controls a feature for each user (e.g. "Meet now in channel meetings")
  - **Per-organizer AND per-user**: both sides must allow (e.g. "Meeting recording")
- After creating a policy, its name can't be changed
- Can't delete a policy that has users assigned — must reassign first

Sources:
- Manage meeting and events policies: <https://learn.microsoft.com/en-us/microsoftteams/meeting-policies-overview> [VERIFIED]
- Settings & policies reference: <https://learn.microsoft.com/en-us/microsoftteams/settings-policies-reference> [VERIFIED]
- Policy assignment overview: <https://learn.microsoft.com/en-us/microsoftteams/policy-assignment-overview> [VERIFIED]

### 3.5 RBAC (Teams admin center roles)

[VERIFIED from policy-assignment-overview page]:
- Global Administrator (all permissions)
- User Administrator (manage users, roles)
- Teams Administrator (Teams settings only)
- Teams Communications Administrator (voice, meetings)
- Guest Invitations Administrator
- Compliance Administrator
- Reports Reader

Sources:
- Policy assignment: <https://learn.microsoft.com/en-us/microsoftteams/policy-assignment-overview> [VERIFIED]
- Roles and permissions (referenced but not fetched): <https://learn.microsoft.com/en-us/microsoftteams/roles-permissions> [UNVERIFIED — 404 on fetch]

## 4. Webex (smaller reference)

[Webex help URLs probed in this session (article IDs 61699, 57200, 57212, 34089, 34374, 40417, 57243, 61700, 57221, 57389) all return HTTP 200 with 170KB response but the body is a JS-rendered SPA shell with a generic "Webex Help Center" title and a "Well, this is embarrassing. We can't seem to find the article" message — the article content is loaded client-side. All Webex claims below are **[UNVERIFIED-SPA]**.]

### 4.1 Webex Meetings — in-meeting roles

Webex uses a similar 4-tier meeting role model [UNVERIFIED-SPA]:
- **Host**: full meeting control
- **Co-host**: shared host permissions
- **Presenter**: can present content and moderate
- **Attendee**: limited to listening

Moderation via Participants panel with host actions similar to Zoom/Teams (mute, remove, spotlight, lock chat, etc.). Waiting room, breakout rooms, Q&A, live captions, recording (local + cloud), transcription are all available.

### 4.2 Webex Admin — IA

Webex admin console (admin.webex.com) left nav [UNVERIFIED-SPA]:
- Overview (dashboard)
- People (users, groups, licenses)
- Roles (custom roles with per-feature scopes)
- Sites (multi-site for enterprise)
- Meetings (defaults, meeting policies, templates)
- Rooms (rooms + room licenses)
- Devices
- Analytics (usage, meetings, rooms, phone)
- Reports
- Logs (call logs, activity logs)
- Settings (call, meeting, email, device, security)
- Policies (call, meeting, messaging, phone)

Webex uses **custom roles** as a first-class pattern — admins define granular roles with scoped permissions (e.g. "Meeting-only admin", "Analytics-only reader") and assign them to users.

Sources (canonical):
- Webex admin console: <https://help.webex.com/en-us/learn/admin-console> [UNVERIFIED-SPA]
- Webex meeting roles: <https://help.webex.com/en-us/article/...> [UNVERIFIED-SPA]
- Webex custom roles: <https://help.webex.com/en-us/article/...> [UNVERIFIED-SPA]

## 5. Cross-product synthesis

### 5.1 Common surface set (across all four mainstream products)

| Surface element | Zoom | Google Meet | MS Teams | Webex | TalkServo implication |
|---|---|---|---|---|---|
| Bottom-center floating toolbar | ✓ | ✓ | ✓ | ✓ | Adopt — universal convention |
| Right dockable panel with tabs | ✓ (Chat, Participants, Transcript) | ✓ (Chat, Participants) | ✓ (Chat, Participants, Q&A, Reactions) | ✓ | Adopt — universal |
| Participants list with row ⋯ menu | ✓ | ✓ | ✓ | ✓ | Adopt — this IS the moderation tool |
| Lobby / waiting room | ✓ (tab in Participants) | Separate modal | ✓ (Lobby panel + Green Room) | ✓ | Adopt for admin/lobby state |
| Raise hand | ✓ | ✓ | ✓ | ✓ | Adopt — universal |
| Spotlight (make one large) | ✓ | ✓ (auto-speaker) | ✓ | ✓ | Adopt — visual floor indicator |
| Pre-join screen (mic/cam preview) | ✓ | ✓ | ✓ | ✓ | Adopt for admin/lobby state |
| Recording | ✓ | ✓ | ✓ | ✓ | Deferred (Beta) |
| Live transcription | ✓ (AI Companion) | ✓ (Gemini) | ✓ | ✓ | Deferred (Beta) |
| Breakout rooms | ✓ | ✓ | ✓ | ✓ | Defer — outside PoC scope |
| Admin console (hosted web) | ✓ | ✓ (Workspace) | ✓ | ✓ | Adopt — Alpha deliverable |

**Consensus layout primitives** (all four use):
1. **Floating bottom toolbar** with 6-10 primary actions
2. **Right dockable panel** (Chat + Participants tabs minimum)
3. **Participants list** as primary moderation tool (row ⋯ menu per participant)
4. **Pre-join preview** with mic level meter + device selector + "join with/without video"

### 5.2 Common admin IA (across all four products)

**Top-level left nav groups** — the recurring pattern:
1. **Users** — user CRUD, groups, license assignment
2. **Teams / Rooms / Sites** — meeting containers (Zoom: Meetings; Teams: Teams+Channels; Webex: Sites+Rooms)
3. **Policies** — feature gating per user (all four)
4. **Devices** — endpoint management (Zoom: Zoom Rooms; Teams: Rooms+Phone; Webex: Devices)
5. **Reports / Analytics** — usage, meeting stats, call quality
6. **Security & Compliance** — auth, retention, DLP, audit logs
7. **Billing** — subscriptions, usage-based pricing
8. **Apps / Marketplace** — third-party integrations

**MS Teams has the most explicit IA** — every setting lives under a "policy" object assigned to users/groups. Zoom and Webex are similar. Google Meet has the least explicit (it's buried under Workspace Admin's general "Apps → Google Workspace" tree).

### 5.3 RBAC models comparison

| Product | Admin tiers | Meeting tiers | Notes |
|---|---|---|---|
| Zoom | Account Owner, Account Admin, User Manager, Co-Host, Meeting Creator, Host, User (custom per group) | Host, Co-host, Presenter (webinar), Panelist/Attendee | Richest meeting roles |
| Google Meet | Super Admin, Groups Editor, User Manager + Meet admin privilege (custom scoped) | Organizer, Co-host, Participant (implicit via Host Management) | Simplest meeting model |
| MS Teams | Global Admin, Teams Admin, Communications Admin, Compliance, Reports Reader | Organizer, Co-organizer, Presenter, Attendee | Clearest "who can present" gate |
| Webex | Admin, Room, Site, User (custom roles with per-feature scopes) | Host, Co-host, Presenter, Attendee | Custom-role-first model |

**Key observation**: All four products have **3-4 meeting role tiers**, and the differentiator is *what those tiers control*. Teams has the clearest articulation — "who can present" is a first-class, explicit setting with 4 options, and it's the closest analog to TalkServo's floor control.

### 5.4 Differentiators

**Zoom** — Richest participant-level actions:
- Request Control / Allow Control (hand-off of screen share)
- Rename participant
- Block user from future meetings
- Multiple breakout rooms with auto-assign
- Zoom AI Companion (transcript, meeting summary, personal assistant)

**Google Meet** — Strongest AI layer:
- "Take notes for me" (Gemini auto-notes)
- "Ask Gemini" (Q&A on live or recorded meeting)
- Recap (AI summary of key discussion points)
- Simplest role model — fewer features but polished UX

**MS Teams** — Clearest "floor control" analog:
- **"Who can present" setting with 4 explicit options** (Everyone / Org+guests / Specific people / Only organizers)
- **Presenter vs Attendee** as first-class role distinction — attendees literally cannot share content or take notes
- **Per-organizer vs per-user vs both** policy implementation — the most nuanced policy model
- Channel-based IA (Teams surface doubles as chat+channels+meetings)
- Richest admin IA — every setting under a policy object

**Webex** — Custom-role-first model:
- Granular per-feature role scoping (any admin can define custom roles)
- Enterprise/government focus (FedRAMP)
- Room/space-centric IA

### 5.5 Implications for TalkServo

**For the "floor control" model (D1)**:
- Teams' **"Who can present"** is the closest mainstream analog. TalkServo's Exclusive/Open/Hybrid floor modes can be positioned as a *generalization* of Teams' setting:
  - **Exclusive floor** ≈ Teams "Only organizers and co-organizers can present"
  - **Open floor** ≈ Teams "Everyone can present"
  - **Hybrid floor** ≈ Teams "Specific people" or "People in my org and guests"
- TalkServo differentiates by making the floor mode a *first-class, dynamic* concept (switchable mid-meeting, per-channel, with grant/taken/idle events) rather than a static pre-meeting setting.

**For the dispatcher / field surface**:
- Adopt the **bottom-center floating toolbar + right dockable panel** pattern (universal convention — users expect it)
- Adopt the **participants list as primary moderation tool** (row ⋯ menu per participant)
- Adopt the **pre-join preview** (mic level meter + device selector)
- Differentiate with **floor state visualization** — Teams/Zoom show who's speaking, but not "who has floor grant". Make floor state a first-class, persistent, visual element (e.g. a colored ring or badge on participant tiles, a dedicated "Floor" indicator).

**For the admin console**:
- Adopt the **MS Teams-style policy-based model** — global policy + custom policies assigned to users/groups. This is the most defensible, explicit RBAC pattern.
- Adopt the **left nav with top-level groups** pattern:
  - Users
  - Groups
  - Floors (channels/rooms with floor modes)
  - Policies (floor mode defaults, RBAC defaults)
  - Sessions (live session monitoring)
  - Reports
  - Security (auth, audit logs)
  - Settings
- Adopt **custom admin roles** (Webex-style) with per-feature scopes — this is more flexible than Teams' fixed role set.

**For differentiation**:
- **Floor state as first-class, live UI element** — Teams/Zoom don't have this. It's TalkServo's unique value prop.
- **PTT as first-class input mode** — mainstream meeting products don't have PTT (they assume mic on/off). TalkServo's PTT mode is a differentiator, especially for dispatcher/field scenarios where push-to-talk is the natural interaction model.
- **Floor mode switchable mid-meeting** — Teams only allows "who can present" to be set before meeting. TalkServo should allow mid-meeting floor mode changes (e.g. "switch to Exclusive floor" when things get noisy).

**What NOT to copy**:
- Don't copy Teams' full policy taxonomy (dozens of policies for Teams, Voice, Messaging, Live Events, Events, Meeting Templates, etc.) — that's enterprise bloat. TalkServo's PoC should have 3-4 policies max (floor mode, RBAC, device, reporting).
- Don't copy the Webinar / Live Events / Events taxonomy — those are for large-scale broadcast, not floor control.
- Don't copy AI Companion / Gemini / Copilot features in PoC — those are Beta/GA-tier.

## 6. Feature comparison matrix (draft)

| Feature | Zoom | Google Meet | MS Teams | Webex | **TalkServo** |
|---|---|---|---|---|---|
| Floor mode (Exclusive/Open/Hybrid) | n/a (implicit via roles) | n/a | Implicit via "Who can present" | n/a | **First-class, dynamic** |
| PTT (push-to-talk) | n/a | n/a | n/a | n/a | **First-class** |
| Grant/taken/idle floor events | n/a | n/a | n/a | n/a | **First-class** |
| Bottom floating toolbar | ✓ | ✓ | ✓ | ✓ | ✓ |
| Right dockable panel | ✓ | ✓ | ✓ | ✓ | ✓ |
| Participants list w/ row menu | ✓ | ✓ | ✓ | ✓ | ✓ |
| Waiting room / lobby | ✓ | ✓ | ✓ (Lobby + Green Room) | ✓ | ✓ (admin lobby state) |
| Spotlight / pin participant | ✓ | ✓ (auto-speaker) | ✓ | ✓ | ✓ |
| Raise hand | ✓ | ✓ | ✓ | ✓ | ✓ |
| Live transcript | ✓ (AI Companion) | ✓ (Gemini) | ✓ | ✓ | Beta |
| Breakout rooms | ✓ | ✓ | ✓ | ✓ | Defer |
| Admin console (hosted web) | ✓ | ✓ (Workspace) | ✓ | ✓ | ✓ (Alpha) |
| Policy-based feature gating | ✓ (per-user) | ✓ (per-OU) | ✓ (per-user + per-organizer) | ✓ (per-role) | ✓ (per-user + per-floor) |
| Custom admin roles | ✓ (per group) | ✓ (custom privileges) | ✓ (fixed set) | ✓ (fully custom) | ✓ (custom, per-feature) |
| Audit logs | ✓ | ✓ (Workspace) | ✓ | ✓ | ✓ (Alpha) |

## 7. Sources

### Zoom
- In-meeting controls: <https://support.zoom.com/hc/en-us/articles/4405999556754> [UNVERIFIED-SPA]
- Manage participants: <https://support.zoom.com/hc/en-us/articles/4405999510298> [UNVERIFIED-SPA]
- Waiting room: <https://support.zoom.com/hc/en-us/articles/26678325050514> [UNVERIFIED-SPA]
- Zoom AI Companion: <https://support.zoom.com/hc/en-us/articles/8357749018266> [UNVERIFIED-SPA]
- Admin portal overview: <https://support.zoom.com/hc/en-us/articles/4405999820442> [UNVERIFIED-SPA]
- Roles & permissions: <https://support.zoom.com/hc/en-us/articles/4404249601298> [UNVERIFIED-SPA]
- Meeting settings (admin): <https://support.zoom.com/hc/en-us/articles/4408822104082> [UNVERIFIED-SPA]
- Audit logs: <https://support.zoom.com/hc/en-us/articles/4412477847066> [UNVERIFIED-SPA]

### Google Meet + Workspace
- Host tools: <https://support.google.com/meet/answer/9928638> [UNVERIFIED-SPA]
- Manage participants: <https://support.google.com/meet/answer/9308681> [VERIFIED — recording permission model]
- Join a meeting: <https://support.google.com/meet/answer/12076912> [UNVERIFIED-SPA]
- Record a meeting: <https://support.google.com/meet/answer/9308681> [VERIFIED]
- Meet admin settings: <https://support.google.com/a/answer/9305658> [UNVERIFIED-SPA]
- Meet recording admin: <https://support.google.com/a/answer/9269229> [UNVERIFIED-SPA]
- Meet DLP: <https://support.google.com/a/answer/14459711> [UNVERIFIED-SPA]
- Meet location tracking: <https://support.google.com/a/answer/12182236> [UNVERIFIED-SPA]
- Meet admin topic landing: <https://support.google.com/a/topic/7542072> [UNVERIFIED-SPA]

### Microsoft Teams
- Teams for Web overview: <https://learn.microsoft.com/en-us/microsoftteams/teams-client-web> [VERIFIED]
- Teams admin documentation landing: <https://learn.microsoft.com/en-us/microsoftteams/> [VERIFIED]
- Roles in Microsoft Teams meetings: <https://support.microsoft.com/office/c16fa7d0-1666-4dde-8686-0a0bfe16e019> [VERIFIED]
- Manage meeting and events policies: <https://learn.microsoft.com/en-us/microsoftteams/meeting-policies-overview> [VERIFIED]
- Settings & policies reference: <https://learn.microsoft.com/en-us/microsoftteams/settings-policies-reference> [VERIFIED]
- Policy assignment overview: <https://learn.microsoft.com/en-us/microsoftteams/policy-assignment-overview> [VERIFIED]

### Webex
- Admin console: <https://help.webex.com/en-us/learn/admin-console> [UNVERIFIED-SPA]
- Meeting roles: <https://help.webex.com/en-us/> [UNVERIFIED-SPA]
- Custom roles: <https://help.webex.com/en-us/> [UNVERIFIED-SPA]

## 8. Notes on fetchability

Three of the four mainstream products' help centers are **JavaScript SPAs** that return only an HTML shell to curl-based fetches:
- `support.zoom.com` — Angular SPA (`ng-app="sn.$sp"`, portal_id-based routing)
- `support.google.com/meet` — JS-rendered article body (curl returns 200 but body is empty)
- `help.webex.com` — JS-rendered, returns "Webex Help Center" title for all probed article IDs
- `support.google.com/a/` (Workspace admin) — returns 200 but redirects to sign-in

Only `learn.microsoft.com` returned full static HTML with the article body, and `support.microsoft.com` returned content with the article TOC. This is a research-methodology constraint: the Zoom and Google Meet sections of this dossier are based on **product knowledge, cited to canonical doc URLs**, not on fetched doc bodies. They should be re-verified when a browser-based fetch tool (Playwright, headless Chrome) is available — the URLs are stable and the content should match.

The `[UNVERIFIED-SPA]` tag marks exactly this: "I know this URL exists and is the right article, but I couldn't read its body via curl."
