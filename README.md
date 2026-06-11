# Increment Tracker

A desktop app for engineering teams that plan in ~3-month increments (≈6 × 2-week
sprints) and track delivery through Jira epics. It answers one question fast:

> **"How are we doing on the epics and issues we committed to in this increment?"**

Built with **Tauri 2** (desktop shell), **Rust** (Jira integration, domain math,
SQLite cache, keychain auth) and **React + TypeScript** (UI). The full product and
technical specification lives in [docs/SPEC.md](docs/SPEC.md).

## Highlights

- **Increments defined by JQL** — e.g. `project in (ABC, XYZ) AND fixVersion = "Increment 25"`;
  works with Jira Data Center (Bearer PAT) and Jira Cloud (email + API token), auto-detected.
- **Binary, SP-weighted progress** — done SP ÷ total SP, no partial credit; in-flight
  work is shown as a hatched segment, never blended into the number.
- **Spillover is a first-class signal** — per-sprint commitment vs done at close,
  chronic offenders (2+ sprints), carried-forward SP, epics carried across increments.
- **One Gantt, three charts, seven insights max** — epic timeline with a today line,
  burn-up with a scope line, sprint completion bars, and a hard-capped insight feed.
- **Local-first** — everything synced is cached in SQLite (instant open, offline-friendly);
  the PAT lives only in the OS keychain and never touches disk or the frontend.

## Repository layout

```
frontend/    React + TypeScript UI (Vite, TanStack Query, Zustand, Recharts)
src-tauri/   Rust backend (Tauri shell, hexagonal-lite architecture)
  src/domain/    pure core: models, progress/spillover math, insights, chart series
  src/jira/      adapter: REST client, auth detection, wire DTOs, field discovery
  src/store/     adapter: SQLite cache + OS-keychain secrets
  src/commands/  thin Tauri command handlers
docs/SPEC.md     product & technical specification
```

Frontend ↔ backend types are generated from the Rust structs via **ts-rs**
(`frontend/src/api/generated/`); the frontend never re-derives a business number.

## Development

Prerequisites: Rust (stable), Node 20+, and the
[Tauri system dependencies](https://tauri.app/start/prerequisites/) for your OS.

```bash
# install JS dependencies
npm install                 # root: Tauri CLI
npm install --prefix frontend

# run the app in dev mode (starts Vite + the Tauri window)
npm run dev

# backend tests (60 domain/store/mapper tests)
cargo test --manifest-path src-tauri/Cargo.toml

# frontend tests + typecheck + production build
npm run test --prefix frontend
npm run build --prefix frontend

# regenerate TypeScript bindings after changing Rust DTOs
TS_RS_EXPORT_DIR=../frontend/src/api/generated \
  cargo test export_bindings --manifest-path src-tauri/Cargo.toml

# production bundle (dmg/msi/AppImage)
npm run build
```

## First run

1. **Settings → Jira connection**: base URL, username/email, PAT → *Test connection* → *Save*.
2. **Settings → Projects**: load and select your projects.
3. **Settings → Increments**: *New increment* — name it (e.g. "Increment 25"), adjust
   the suggested JQL, *Validate*, set start/end dates, save.
4. Press **Sync** in the top bar. The dashboard, epics, sprints and spillover views
   all answer from the local cache afterwards.

## Notes & assumptions

- "Planned fix version" is assumed to be the standard `fixVersion` field; teams that
  use something else simply change the increment JQL (that's why increments are JQL-defined).
- Scrum boards are assumed (sprint history comes from the Sprint field + changelog);
  Kanban-only teams are out of scope for v1.
- The app is strictly **read-only** against Jira — a read-scope PAT is sufficient.
- Descoped issues (Won't Do / Duplicate / Cannot Reproduce) leave both the numerator
  and denominator of progress and are listed separately, so scope changes stay visible.
