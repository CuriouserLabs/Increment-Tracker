# Increment Tracker — Product & Technical Specification

A desktop app (Tauri + Rust + React/TypeScript) for engineering teams that plan in
3-month increments (~6 × 2-week sprints) and want one answer fast:

> **"How are we doing on the epics and issues we committed to in this increment?"**

---

## 1. Product Scope

### v1 — must ship

| Capability | Notes |
|---|---|
| Connect to Jira (base URL + username + PAT) | Supports both Jira Data Center (Bearer PAT) and Jira Cloud (Basic email+API-token). Auto-detected, overridable. |
| Define increments via JQL | e.g. `project in (ABC, XYZ) AND issuetype = Epic AND fixVersion = "Increment 25"` |
| Increment dashboard | Overall progress, Gantt of epics, burn-up, spillover summary |
| Epic list + epic detail drill-down | Story-point progress, child issues grouped by sprint/status |
| Sprint view with spillover detection | Issues that crossed sprint boundaries, carried-forward work |
| Local cache (SQLite) + manual/auto refresh | App is usable offline with last-synced data |
| At-risk insights (in-app only) | Epic at risk, increment off track, repeated spillover |

### Postponed (v2+)

- Multiple Jira connections / multiple teams side by side
- Historical increment comparison and velocity trends across increments
- Export (PDF/PNG report for leadership), scheduled email digests
- Native OS notifications (v1 shows insights in-app only — desktop popups from a
  reporting tool are noise)
- Time-in-status / cycle-time analytics, cumulative flow diagrams
- Editing Jira data (the app is strictly **read-only** in v1 — this keeps auth scope,
  trust, and failure modes simple)
- Custom progress weighting models (v1 ships one opinionated model)

### Minimal useful workflow

1. **Settings** → enter base URL, username, PAT → *Test connection*.
2. Pick project(s); the app suggests an increment JQL template; user names the
   increment ("Increment 25") and confirms dates (auto-derived from epic min-start /
   max-end, editable).
3. **Sync** (one click, progress shown). Everything below works off the local cache.
4. **Home** answers "how are we doing" in <5 seconds of looking.
5. Drill: increment → epic → sprint → issue (opens in browser via Jira deep link).

---

## 2. Information Architecture

Sidebar (5 items — deliberately flat):

```
◧ Home          ← the increment dashboard (default screen)
▤ Epics         ← table of committed epics, drill into epic detail
▥ Sprints       ← per-sprint completion + spillover (renamed from "Sprint Spillovers")
⮔ Spillover     ← cross-cutting spillover report (issues AND epics)
⚙ Settings      ← connection, projects, increments, sync
```

Changes from the proposed structure, and why:

- **"Increments" is not a sidebar item.** The active increment is a **global selector
  in the top bar** (a dropdown next to the sync button). Every screen is scoped to one
  increment; making it a page would force users to "go somewhere" to do the most common
  action (switching context). Managing/creating increments lives in Settings.
- **"Sprints" instead of "Sprint Spillovers"** as a primary section: users also need
  plain sprint completion ("how did Sprint 3 go?"), not only the failure cases.
- **"Spillover" stays as its own section** because it's the #1 leadership signal and
  deserves a dedicated, shareable view that aggregates both issue-level (sprint→sprint)
  and epic-level (increment→increment) spillover, plus carried-forward backlog.

Top bar (persistent): increment selector · last-synced timestamp · Sync button ·
global filter chips (project, epic owner).

---

## 3. Jira Data Strategy

### Authentication (assumption stated)

"Username + PAT" means different things per deployment:

- **Jira Data Center / Server**: PATs exist natively → `Authorization: Bearer <PAT>`
  (username not sent; kept for display/`assignee = currentUser()` resolution).
- **Jira Cloud**: "PAT" is an API token → `Authorization: Basic base64(email:token)`.

v1 supports both. Detection: try `GET /rest/api/2/myself` with Bearer; on 401 retry
with Basic. Store the winning mode. A manual override lives in Settings.

### Endpoints used (all standard REST, no apps/plugins required)

| Purpose | Endpoint |
|---|---|
| Validate connection / current user | `GET /rest/api/2/myself` |
| Discover projects | `GET /rest/api/2/project` |
| Discover fields (story points, epic link, sprint) | `GET /rest/api/2/field` |
| Search issues by JQL (paged) | `GET /rest/api/2/search?jql=...&fields=...&expand=changelog` (Cloud v3 uses `POST /rest/api/3/search/jql`) |
| Sprint details (dates, state) | `GET /rest/agile/1.0/sprint/{id}` |
| Board sprints (optional enrichment) | `GET /rest/agile/1.0/board/{id}/sprint` |
| Statuses → status categories | `GET /rest/api/2/status` |

**Field discovery is mandatory, not optional.** Story Points, Epic Link, and Sprint are
custom fields with instance-specific IDs (`customfield_10016` etc.). On first sync the
app calls `/rest/api/2/field`, matches by name (`Story Points`, `Story point estimate`,
`Epic Link`, `Sprint`), caches the mapping, and lets the user correct it in Settings.

### Discovery flow per sync

1. **Epics of the increment** — run the increment's JQL (user-configured, defaulted to
   `project in (...) AND issuetype = Epic AND fixVersion = "<increment>"`).
   *Assumption:* "planned fix version" is the standard `fixVersion` field. If a team
   uses a custom field instead, the JQL template absorbs that difference — this is
   exactly why increments are JQL-defined rather than hard-coded.
2. **Issues under each epic** — one JQL query for all epics at once:
   - Data Center: `"Epic Link" in (ABC-1, ABC-2, ...)`
   - Cloud (company-managed): `parent in (ABC-1, ABC-2, ...)`
   The clause template is configurable; default chosen by detected deployment type.
3. **Sprint membership** — from the Sprint custom field on each issue. Jira returns
   the *full sprint history* (all closed sprints + active sprint) in that field, which
   is precisely what spillover detection needs. Sprint ids are then resolved via the
   Agile API for names/dates/state, cached.
4. **Status transitions** — `expand=changelog` on the search. We extract `status`
   items (from → to, timestamp) and `Sprint` items. This gives:
   - when an issue became Done (for burn-up curves),
   - reopen events (Done → not-Done),
   - sprint reassignment events (for "moved out mid-sprint" vs "added late").
   *Changelog paging caveat:* Cloud caps embedded changelogs at 100 entries/issue;
   issues that hit the cap get a follow-up `GET /issue/{key}/changelog` call. Rare in
   practice for sprint-sized issues.
5. **Completed vs incomplete** — by **status category**, never by status name.
   Jira guarantees every status maps to `new` / `indeterminate` / `done`. Done =
   status category `done`. This survives custom workflows ("Deployed", "Won't Do" —
   see §4 for the Won't Do carve-out).

### Deriving progress with no "percent complete" field (opinionated)

**Progress = story points in Done status category ÷ total story points. Binary. No
partial credit for in-progress work.**

Why: every partial-credit scheme (50% for in-progress, etc.) makes the number
unfalsifiable and erodes trust the first time a "90% done" epic slips. A binary
SP-weighted number is the same arithmetic a delivery lead would do by hand, so they
trust it. In-progress work is shown *alongside* the number (a hatched segment on the
bar), not blended into it.

**Unestimated issues:** counted at the epic's median issue estimate for *totals*
(so the denominator isn't a lie) but flagged with a badge ("3 unestimated"). An epic
with >30% unestimated SP shows a data-quality warning instead of pretending precision.

### Sync & rate limiting

- Full sync = ~3–6 JQL pages for a typical increment (60–80 epics worth of issues at
  100/page with changelog). Incremental sync afterwards: `updated >= "<last sync>"`
  re-fetches only changed issues.
- Requests are serialized with modest concurrency (4), exponential backoff on 429,
  honoring `Retry-After`.

---

## 4. Progress Calculation Logic

All formulas use **story points (SP)** as the unit. Issue counts are shown only as
secondary context. Notation: for an issue *i*, `sp(i)` is its story points (or imputed
median, flagged), `done(i)` is true iff status category = done **and** resolution is
not "Won't Do"/"Duplicate"/"Cannot Reproduce" (those are *descoped*, removed from both
numerator and denominator, and listed in a "Descoped" drawer so the scope change is
visible, not silent).

### Epic progress

```
epic_progress = Σ sp(i) for done child issues / Σ sp(i) for all child issues
```

Shown with three segments: **Done** (solid) · **In progress** (hatched, status
category = indeterminate) · **Not started**. Blocked issues (flag or `status in
configured blocked-statuses`) tint their segment red but stay in their category.

If an epic has zero child issues, fall back to the epic's own SP and status
(0% or 100%) and badge it "no breakdown" — that's a planning smell worth surfacing.

### Increment progress

```
increment_progress = Σ done SP across all committed epics / Σ total SP across all committed epics
```

SP-weighted, *not* an average of epic percentages — a 100-point epic must matter more
than a 5-point one. "Committed epics" = the epics matched by the increment JQL at sync
time; epics that later disappear from the JQL result are kept and badged **Removed
from plan** rather than silently dropped (scope changes must be visible).

### Expected progress (drives at-risk detection)

```
expected_progress = clamp((today − increment_start) / (increment_end − increment_start), 0, 1)
variance = increment_progress − expected_progress
```

Linear time expectation — deliberately naive and explainable. Per-epic the same
formula uses the epic's own start/end dates. (v2 could use historical velocity; v1
favors a model anyone can recompute mentally.)

### Sprint progress & completion

For sprint *S* with committed set `C(S)` = issues in S at sprint start (derived from
the Sprint-field changelog: in the sprint when it started, or added in the first 24h):

```
sprint_completion = Σ sp(i), i ∈ C(S) done before sprint end / Σ sp(i), i ∈ C(S)
```

Issues **added mid-sprint** are shown separately ("+8 SP added") and excluded from the
commitment denominator — otherwise scope-adding sprints look like failing sprints.

### Spillover

- **Issue spillover**: issue *i* spilled if it appears in a *closed* sprint where it
  was not done at sprint close. `spill_count(i)` = number of such sprints (the Sprint
  field's closed-sprints list makes this cheap). An issue in 3 closed sprints unfinished
  is a much stronger signal than one normal carry-over — show the count.
- **Sprint spillover rate**:

  ```
  spillover_rate(S) = SP committed to S but not done at close / SP committed to S
  ```

- **Epic spillover (increment level)**: an epic spilled if its fixVersion history (or
  presence in a previous increment's saved definition) shows a prior increment, or its
  end date precedes increment start while still incomplete. Badged **Carried over** with
  the originating increment named.
- **Carried-forward work** (Home headline number): SP entering the current sprint that
  were committed to an earlier sprint and remain undone.

### Status edge cases

| Case | Treatment |
|---|---|
| Done issues | Count fully, dated by transition-to-done timestamp (for burn-up) |
| In progress | 0% credit; shown as hatched segment + "in flight" SP |
| Blocked | 0% credit; red tint; counted in "blocked SP" insight feed |
| Reopened (done → not done) | Removed from done SP as of reopen date (burn-up can dip — honest data); badge ↩ Reopened |
| Won't Do / descoped | Removed from numerator & denominator; listed in Descoped drawer |
| Unestimated | Imputed at epic median for totals; badged; >30% imputed ⇒ data-quality warning |

---

## 5. Dashboard Design (Home)

Layout — one screen, no scrolling on a 13" laptop, four zones:

```
┌────────────────────────────────────────────────────────────────────┐
│  Increment 25 ▾        synced 12 min ago   [Sync]   filters: ⬡ ⬡  │
├──────────────┬──────────────┬───────────────┬──────────────────────┤
│ Progress     │ Pace         │ Spillover     │ Scope                │
│ 58% (210/362)│ −9% vs plan  │ 31 SP carried │ +14 SP / −8 SP       │
│ ▓▓▓▓▓░hatch░ │ ⚠ behind     │ ▲ 3 chronic   │ since planning       │
├──────────────┴──────────────┴───────────────┴──────────────────────┤
│  EPIC TIMELINE (Gantt)                                  ~55% height│
│  Sprint 1 │ Sprint 2 │ Sprint 3 │ Sprint 4 │ Sprint 5 │ Sprint 6   │
│  ────────────────────────▼ today ──────────────────────────────    │
│  Epic A (Kasun)   ▓▓▓▓▓▓▓▓▓░░░░░  72%                              │
│  Epic B (Nadee)   ▓▓▓░░░░░░░░░░░  23% ⚠                            │
│  Epic C (Ruwan)  ↩▓▓▓▓▓░░░░░░░░░  41% carried over                 │
├──────────────────────────────────┬─────────────────────────────────┤
│  BURN-UP (SP done vs scope vs    │  SPRINT COMPLETION              │
│  ideal line, by sprint)          │  bars: done vs spilled per      │
│                                  │  sprint, S1…S6                  │
└──────────────────────────────────┴─────────────────────────────────┘
```

The chart set (only four, deliberately):

1. **Epic timeline / Gantt** *(the centerpiece)* — one row per epic across the
   increment's sprint columns; bar position = epic start/end dates, fill = SP done %,
   a vertical "today" line, owner avatar, risk/carry-over badges. **Why:** it merges
   the three questions leadership actually asks — what's planned, where are we in
   time, and how complete is each thing — into one glance. A bar whose fill is well
   left of the today-line *is* the at-risk visualization.
2. **Burn-up** (not burn-down) — cumulative done SP per sprint vs an ideal line vs a
   **total-scope line**. **Why burn-up:** the scope line makes scope creep visible as a
   rising ceiling; burn-down hides whether you slipped because you slowed down or
   because scope grew. Reopens show as honest dips.
3. **Sprint completion comparison** — per closed sprint, a stacked bar: done SP vs
   spilled SP, with mid-sprint additions as a thin marker. **Why:** trend of the
   team's commitment reliability; three rising spill bars predict the increment
   outcome better than any single number.
4. **KPI strip** (not a chart): Progress %, Pace vs plan, Carried-forward SP, Scope
   change. Four numbers, each clickable to its detail view.

Explicitly rejected for v1: pie charts (status pies answer no decision), cumulative
flow (expert tool, high cognitive load), per-person workload charts (invites misuse as
a performance metric — owner *bottleneck* shows up in insights instead, framed as a
WIP problem).

---

## 6. UI/UX Behavior

**Interaction spine: summary → drill → Jira.** Every number is clickable; the deepest
level always offers "Open in Jira" (deep link `<base>/browse/KEY`) rather than
re-implementing Jira's issue view.

- **Select an increment:** top-bar dropdown, persisted per window. Switching swaps the
  whole app's scope; cached increments switch instantly.
- **Drill into an epic:** click a Gantt row or Epics-table row → epic detail: header
  (owner, dates, SP bar, risk badge), child issues grouped by **sprint** (toggle:
  group by status), per-sprint mini-progress.
- **Inspect a sprint:** Sprints section, or click a sprint column header in the Gantt.
  Sprint view = committed vs added vs done vs spilled, with the spilled list on top.
- **See delayed/spilled work quickly:** red ⮔ badges everywhere an affected item
  appears; the Spillover section lists worst offenders first (by spill count, then SP).

**Epics table columns (v1):**
`Epic key · Name · Owner · SP done/total (bar) · Pace (▲/▼ vs expected) · Sprint span · Spill ⮔ · Status`

**Issue table columns:**
`Key · Summary · SP · Status (category-colored chip) · Sprint · Spill count ⮔ · Flags (blocked ⛔ / reopened ↩ / added-late +)`

**Filters:** chips above tables for project, epic owner, status category, "only
at-risk", "only spilled". Filter state is per-view and shown as removable chips —
never hidden behind a modal.

**Badges (one consistent vocabulary):**
⚠ at-risk · ⮔ spilled (with count) · ↩ reopened · ⛔ blocked · ＋ added mid-sprint ·
∅ unestimated · ⤴ carried over (epic).

**Executive-friendly rules:** numbers always paired with their fraction
(`58% — 210/362 SP`); no raw Jira jargon on Home (no "status category", no
customfield ids); empty/error states say what to do ("PAT expired — update in
Settings"), not stack traces.

---

## 7. Technical Architecture

**Pattern: hexagonal-lite.** A pure domain core (Rust) with Jira and SQLite as
adapters; the React UI is a thin renderer over Tauri commands. The single most
important rule: **all progress/spillover math lives in Rust domain code with no I/O,
so it is unit-testable with fixture JSON and reusable if a CLI/CI reporter is ever
wanted.**

```
Rust (src-tauri/src/)
├── main.rs / lib.rs            # Tauri setup, command registration
├── commands/                   # Tauri command handlers (thin: parse, call, serialize)
│   ├── connection.rs           #   test_connection, save_credentials
│   ├── sync.rs                 #   sync_increment (emits progress events)
│   ├── queries.rs              #   get_dashboard, get_epic_detail, get_sprint_detail…
│   └── settings.rs
├── jira/                       # ADAPTER: Jira REST integration
│   ├── client.rs               #   reqwest client, auth modes, retry/backoff/429
│   ├── auth.rs                 #   Bearer vs Basic detection
│   ├── search.rs               #   paged JQL search + changelog follow-up
│   ├── agile.rs                #   sprint/board endpoints
│   ├── fields.rs               #   custom-field discovery & mapping
│   └── dto.rs                  #   raw wire types (serde) — never leak past mapper.rs
├── domain/                     # PURE CORE: no I/O, no serde-json, fully unit-tested
│   ├── model.rs                #   Increment, Epic, Issue, Sprint, StatusCategory
│   ├── mapper.rs               #   jira::dto → domain (incl. changelog parsing)
│   ├── progress.rs             #   epic/increment/sprint progress formulas (§4)
│   ├── spillover.rs            #   spill detection & rates
│   ├── insights.rs             #   at-risk rules (§10)
│   └── timeline.rs             #   Gantt/burn-up series computation
├── store/                      # ADAPTER: persistence
│   ├── db.rs                   #   rusqlite + migrations
│   ├── cache.rs                #   issue/sprint cache, incremental-sync watermarks
│   └── secrets.rs              #   keyring crate (macOS Keychain / Win Credential Mgr / Secret Service)
└── error.rs                    # one AppError enum → user-meaningful messages

React (src/)
├── app/                        # shell: router, sidebar, top bar, increment selector
├── api/                        # typed invoke() wrappers + TanStack Query hooks
│   └── types.ts                # generated from Rust via ts-rs (single source of truth)
├── features/
│   ├── home/                   # KPI strip, Gantt, burn-up, sprint-completion chart
│   ├── epics/                  # table + epic detail
│   ├── sprints/
│   ├── spillover/
│   └── settings/
├── components/                 # ProgressBar, Badge, DataTable, FilterChips, EmptyState
└── lib/                        # formatting, date helpers
```

**Decisions & trade-offs:**

- **Chart-ready data is computed in Rust** (`timeline.rs` returns plain series:
  `{sprint, doneSp, scopeSp, idealSp}[]`). The UI never re-derives business numbers —
  one implementation of every formula, testable in one place. Trade-off: a Tauri
  round-trip per view; cheap, since it reads SQLite, not Jira.
- **State management:** TanStack Query over the command layer (caching, refetch on
  sync-complete event) + a small Zustand store for UI-only state (selected increment,
  filters). No Redux — there is almost no client-side mutation.
- **Custom Gantt as SVG/divs in React** (rows × sprint columns is a simple grid);
  burn-up/bars via **Recharts**. Off-the-shelf Gantt libraries are heavyweight and
  fight the "fill = progress" design.
- **Sync runs in a Tauri async task**, emitting `sync://progress` events ("Fetching
  epics… 2/4 pages"); UI stays responsive, sync is cancellable.
- **ts-rs** generates TypeScript types from the Rust domain structs — no drift between
  backend and frontend contracts.
- **Testing:** domain = pure unit tests with recorded Jira JSON fixtures (incl. nasty
  changelogs: reopened, multi-spill, added-late); jira adapter = wiremock contract
  tests; UI = Vitest + Testing Library for tables/badges; one Playwright/tauri-driver
  smoke test (settings → mock sync → dashboard renders).

---

## 8. Settings & Configuration

**Connection:** base URL · username/email · PAT (write-only field: settable, testable,
never displayed) · auth mode (auto/Bearer/Basic) · *Test connection* button.

**Projects:** multi-select fetched from `/rest/api/2/project` after connecting.

**Increments:** a named list; each = `{ name, jql, start_date, end_date, status }`.
Dates auto-suggested from epic min/max dates, editable. One increment is "active by
default" on app open.

**Field mapping:** auto-discovered story-points / epic-link / sprint fields shown with
an override dropdown (instances with multiple "Story Points" fields exist).

**Advanced:** epic-children JQL clause template · blocked-status names · auto-sync
interval (default: manual + on-open).

**Credential storage:** PAT goes in the **OS keychain** via the `keyring` crate
(macOS Keychain, Windows Credential Manager, libsecret on Linux) — never in config
files, never in SQLite, never in frontend state (the React app literally cannot read
it; only Rust touches it). Non-secret settings live in a plain JSON config under the
Tauri app-config dir, safe to back up.

**Cache policy:**
- *Cache:* issues, epics, sprints, changelog-derived events, computed snapshots,
  field mappings — all in SQLite under app-data; enables instant open + offline.
- *Never cache:* the PAT (keychain only), other users' Jira avatars beyond session,
  anything from projects the user deselected (deselect ⇒ purge).
- A visible "synced 12 min ago" stamp everywhere, and Settings → "Clear local data".

---

## 9. JQL Support

Increments are **defined by JQL** — this is the core configurability mechanism and
absorbs every instance-specific quirk (custom "planned fix version" fields, multiple
projects, labels-based planning).

- Default template on creation:
  `project in ({projects}) AND issuetype = Epic AND fixVersion = "{increment_name}"`
- Free-text JQL editor with **Validate** (runs the query, shows count + first 10 epics
  before saving) — instant feedback beats syntax help.
- **Multiple projects:** naturally supported by `project in (...)`; the project
  multi-select in Settings just feeds the template.
- **Query presets (v1, lightweight):** saved *filter* presets per view (e.g. "At-risk
  only", "Owner = me") stored locally. Reusable *JQL snippets* shared across
  increments (e.g. the epic-children clause) are a v2 nicety; v1 keeps one JQL per
  increment plus one advanced clause template.
- Guardrail: the app appends nothing hidden to user JQL except `issuetype = Epic` if
  missing (with a notice) — surprising silent rewrites destroy trust in the numbers.

---

## 10. Notifications & Insights (in-app, conservative)

An **Insights panel** on Home (collapsible, badge with count). Rules fire only on
sync, deduplicate, and each card says *why* it fired and links to the evidence.
Hard cap: if more than ~7 insights fire, show the top 7 by SP impact — a wall of
warnings is the same as none.

| Insight | Rule (all thresholds user-tunable, defaults shown) | Why high-signal |
|---|---|---|
| **Epic at risk** | `epic_progress < expected_progress − 15%` AND ≥1 sprint elapsed AND remaining SP > team's median sprint throughput for remaining sprints | Catches "quietly behind" before the last sprint |
| **Sprint spillover warning** | Last closed sprint `spillover_rate > 25%`, or same issue spilled ≥2 sprints | Chronic spill is the strongest predictor of increment miss |
| **Increment off track** | Projected finish (done SP + remaining sprints × median done-SP/sprint) < 85% of scope | One headline projection, recomputed per sync |
| **Owner bottleneck** | One owner holds >40% of remaining SP across ≥2 at-risk epics | Framed as WIP/load problem ("consider rebalancing"), never as performance |
| **Data quality** | >30% imputed SP, or epics with no children, or unmapped fields | Bad inputs silently corrupt every number above |

No OS push notifications in v1 (deferred): a reporting tool interrupting people
breeds resentment and gets disabled; the insight badge on app open is enough.

---

## 11. Data Model Outline (SQLite)

```sql
connections(id, base_url, username, auth_mode)            -- PAT in OS keychain, keyed by id
increments(id, name, jql, start_date, end_date, is_active)
epics(id, key, increment_id, name, owner, sp, start_date, end_date,
      status_category, carried_from_increment, removed_from_plan, synced_at)
issues(id, key, epic_id, summary, sp, sp_imputed, status, status_category,
       resolution, blocked, current_sprint_id, synced_at)
sprints(id, jira_id, name, state, start_date, end_date, board_id)
issue_sprints(issue_id, sprint_id, was_committed, added_mid_sprint,
              done_at_close)                              -- spillover backbone
status_events(issue_id, from_category, to_category, at)   -- burn-up + reopen detection
snapshots(increment_id, taken_at, done_sp, scope_sp, in_progress_sp)
                                                          -- per-sync trend points
sync_state(increment_id, last_full_sync, last_incremental_watermark)
```

`issue_sprints` + `status_events` are the two tables everything interesting derives
from; both are populated from the changelog during sync, so queries never re-parse
changelogs.

---

## 12. Implementation Milestones

| # | Milestone (each ends runnable) | Scope | ~Effort |
|---|---|---|---|
| 0 | **Skeleton** | Tauri + React + router + sidebar; CI; ts-rs pipeline | 2–3 d |
| 1 | **Connect & discover** | Settings: connection, keychain storage, test-connection, project + field discovery | 1 wk |
| 2 | **Sync & cache** | Increment JQL config, full sync (epics→issues→sprints→changelog), SQLite cache, progress events | 1.5 wk |
| 3 | **Domain math + Epics view** | progress.rs/spillover.rs with full unit tests; Epics table + epic detail | 1.5 wk |
| 4 | **Home dashboard** | KPI strip, Gantt, burn-up (from snapshots + status_events), sprint completion chart | 1.5 wk |
| 5 | **Sprints & Spillover views** | sprint detail, spillover report, badges everywhere | 1 wk |
| 6 | **Insights + polish** | insight rules, incremental sync, empty/error states, offline mode, packaging (signed dmg/msi) | 1 wk |

≈ 7–8 weeks for one engineer to a credible v1; milestones 3–4 are the heart and the
right place to spend extra care.

---

## Appendix A — Key Assumptions

1. "Planned fix version" = standard `fixVersion`; deviations handled via increment JQL.
2. One Jira connection, one team, one active increment at a time (v1).
3. Story points readable from a single discovered custom field per instance.
4. Sprint history is recoverable from the Sprint custom field + changelog (true for
   Scrum boards; Kanban-only teams are out of scope for v1).
5. Done means status category `done` with a non-descoping resolution.
6. The app is read-only against Jira; a PAT with read scope suffices.
7. Increment dates: trusted from user config (seeded from epic dates), since Jira has
   no first-class "increment" object.
