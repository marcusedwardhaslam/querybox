---
name: QueryBox Roadmap
description: Priority-first release plan from v0.1.1 to v0.5, personal planning only
type: project
---

# QueryBox Roadmap

**Scope:** Personal planning only. No public-facing commitments.  
**Strategy:** Priority-first — each milestone makes the app meaningfully more useful before moving to polish.  
**Current version:** v0.1.1  
**Windows support:** Parked indefinitely. Mac-only.

---

## Ongoing — Build Pipeline (in progress)

GitHub Releases automation with binary download. Runs alongside all milestones and is a prerequisite for the Mac bundle in v0.5.

---

## v0.2 — Data Editing Completeness

**Theme:** Table  
**Goal:** The table view can handle all day-to-day data work — browse, edit, insert, and navigate to related rows.

| Feature | Notes |
|---|---|
| New row support in table view | Insert rows inline from the table view |
| Foreign key direct navigation | `orders.user_id` → jump to `users` table filtered to that ID |

---

## v0.3 — UI Polish

**Theme:** Feel  
**Goal:** The app feels refined and pleasant — no rough edges in the daily workflow.

| Feature | Notes |
|---|---|
| Sticky column names | Column headers stay visible when scrolling down |
| Syntax highlighting in query editor | Colour-coded SQL in the New Query view |
| Format SQL button | Auto-format/pretty-print the current SQL |
| Schema view / Results view toggle | Switch between schema inspector and query results in the same pane |

---

## v0.4 — Power User Features

**Theme:** Editor  
**Goal:** The SQL editor is genuinely powerful — useful for complex debugging and query tuning.

| Feature | Notes |
|---|---|
| Run only highlighted SQL | Execute the selected portion of the query, not the whole file |
| View raw query panel | Inspect the raw SQL being sent to the database |
| Running queries modal | View and kill long-running queries |

---

## v0.5 — App Features

**Theme:** Platform  
**Goal:** QueryBox feels like a real Mac app — native preferences, distributable, installable.

| Feature | Notes |
|---|---|
| OS-level preferences (⌘,) | Native macOS preferences window |
| Keychain / encoded JSON option | Opt out of keychain; store credentials in encoded JSON instead |
| Mac OS bundled application | Distributable `.app` bundle, enabled by build pipeline |
