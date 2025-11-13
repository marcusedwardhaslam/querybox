# QueryBox — Agent Guide

## Project Overview

QueryBox is a **free and open source** alternative to TablePlus. It is a native SQL GUI built in **Rust** using the **GPUI** framework (from the Zed editor). The goal is to provide a polished, zero-cost database management experience.

## Core Features

- **Browse tables** — view table data with pagination
- **Edit tables** — inline editing of rows with insert/update/delete
- **Query with filters** — filter table results without writing SQL
- **Raw SQL editor** — write and execute arbitrary SQL, display results in a table
- **Export** — export any dataset as CSV, SQL, or JSON
- **Multiple databases** — select and switch between databases on the same connection
- **Schema inspector** — view all tables, columns, types, keys, and indexes

## Tech Stack

- **Language:** Rust
- **UI Framework:** GPUI (https://github.com/zed-industries/zed/tree/main/crates/gpui)
- **Database connectivity:** MySQL (primary target via `mysql_async` or `sqlx`)
- **Dev database:** MySQL 8.0 via Docker Compose (`dev/docker-compose.yml`)

## Project Structure

```
querybox/
├── AGENTS.md          # This file (symlinked to CLAUDE.md)
├── README.md          # Public-facing readme
├── dev/
│   ├── docker-compose.yml   # MySQL 8.0 dev server
│   ├── mysql-init/init.sql  # Seed data (users, orders tables)
│   └── data/                # MySQL data dir (gitignored)
└── src/                     # Rust source (to be created)
    ├── main.rs
    └── ...
```

## Dev Environment

### Database

```sh
cd dev && docker compose up -d
```

- **Host:** localhost:3306
- **Root password:** password
- **App user:** queryuser / querypass
- **Database:** querybox
- **Seed tables:** `users`, `orders`

## Architecture Guidelines

- Use GPUI's retained-mode UI model — components are structs that implement `Render`.
- Keep database I/O async and off the UI thread.
- Separate concerns: connection management, query execution, result rendering, and export are distinct modules.
- Support multiple simultaneous database connections.
- All SQL passed to the database must use parameterized queries to prevent SQL injection.

## Coding Conventions

- Follow standard Rust idioms (`cargo fmt`, `cargo clippy` clean).
- Use `thiserror` for typed errors, `anyhow` only at the application boundary.
- Prefer returning `Result` over panicking.
- Keep GPUI views thin — business logic belongs in models/services.
- Name files and modules in `snake_case`; types in `PascalCase`.

## Key Decisions to Record

When making significant architectural decisions, document them as comments in the relevant module or in this file under a new "## ADRs" section.
