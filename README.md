# hungry-family

A self-hosted collaborative meal planning and grocery shopping app for the family. Built with a Rust/Axum backend, React frontend, and PostgreSQL database.

The core problem it solves: keeping a shared, up-to-date shopping list so you stop defaulting to takeout. Any family member can log in, browse products organized by store aisle, and add items to the active shopping list in real time.

---

## Tech Stack

| Layer | Technology |
|---|---|
| Frontend | React 19 + Vite |
| Backend | Rust (Axum 0.8) |
| Database | PostgreSQL |
| Auth | Session cookies + Argon2id password hashing |
| ORM | SQLx (compile-time verified queries) |

---

## Project Structure

```
.
├── src/                    # Rust backend
│   ├── main.rs             # Server entry point + route registration
│   ├── auth.rs             # Session management, password verification
│   ├── routes.rs           # All API handlers
│   └── bin/
│       └── hash_password.rs  # Helper binary to hash passwords
├── frontend/               # React frontend source
│   └── src/
│       ├── App.jsx         # Main app + all page components
│       ├── AuthContext.jsx # Session state provider
│       ├── LoginPage.jsx   # Login form
│       └── ...
├── migrations/             # SQL migration files (run in order)
│   ├── 0001_init.sql
│   ├── 0002_store_layouts.sql
│   └── 0003_shopping_lists_v2.sql
└── static/                 # Built frontend output (served by Axum)
```

---

## Prerequisites

- **Rust** (stable, 2024 edition) — [rustup.rs](https://rustup.rs)
- **Node.js** v20+ and npm
- **PostgreSQL** (v14+ recommended)

---

## Setup

### 1. Clone and configure environment

```bash
git clone <repo-url>
cd hungry-family
cp .env.example .env   # or create .env manually
```

Create a `.env` file in the project root:

```env
DATABASE_URL=postgres://your_user:your_password@localhost/hungry_family
```

### 2. Set up the database

Create the database and run migrations in order:

```sql
-- In psql or your preferred client:
CREATE DATABASE hungry_family;
```

Then run each migration file against the database:

```bash
psql "$DATABASE_URL" -f migrations/0001_init.sql
psql "$DATABASE_URL" -f migrations/0002_store_layouts.sql
psql "$DATABASE_URL" -f migrations/0003_shopping_lists_v2.sql
```

### 3. Seed initial data

The app requires at least one user and account to log in. Use the `hash_password` helper to generate an Argon2id hash for a password:

```bash
cargo run --bin hash_password -- "your_password_here"
# Prints the hash to stdout
```

Then insert a user and account into the database:

```sql
-- Insert a user record
INSERT INTO users (firstname, lastname, email)
VALUES ('Jane', 'Smith', 'jane@example.com');

-- Insert an account (use the hash printed above)
INSERT INTO accounts (user_id, username, password_hash)
VALUES (
  (SELECT user_id FROM users WHERE email = 'jane@example.com'),
  'jane',
  '$argon2id$v=19$...<paste hash here>...'
);
```

Repeat for each family member who needs an account.

> **Note:** There are no pre-seeded default users — all accounts must be created manually via SQL. Every family member gets the same permissions; there are no admin roles.

### 4. Build the frontend

```bash
cd frontend
npm install
npm run build
cd ..
```

This compiles the React app into `static/`, which Axum serves as static files.

### 5. Run the server

```bash
cargo run
```

The server starts on **http://localhost:800** (port 800 requires `cap_net_bind_service` on Linux — see `.cargo/config.toml` for the runner configuration that handles this automatically on `x86_64-unknown-linux-gnu`).

---

## Development Workflow

For frontend hot-reload during development, run the Vite dev server alongside the Rust backend. The Vite config proxies `/api` requests to the backend at `localhost:800`.

```bash
# Terminal 1 — backend
cargo run

# Terminal 2 — frontend dev server
cd frontend
npm run dev
```

Frontend dev server runs on **http://localhost:5173** by default.

After making frontend changes for production, rebuild:

```bash
cd frontend && npm run build
```

---

## Database Schema Overview

| Table | Purpose |
|---|---|
| `users` | Family member profiles (name, email) |
| `accounts` | Login credentials (username + Argon2id hash) |
| `sessions` | Active session tokens with expiry |
| `stores` | Store definitions (e.g. Woodman's, Costco) |
| `store_layouts` | Aisle/section labels for each store, with sort order |
| `standalone_products` | Global product catalog (name-deduped across stores) |
| `store_products` | Per-store product entries with aisle assignment |
| `store_shopping_lists` | One active list per store at a time |
| `store_shopping_list_items` | Items on a list with quantity |

Key constraints enforced at the database level:
- Only one `active` shopping list per store at a time (partial unique index)
- `quantity` must be > 0
- Product and user display names cannot be blank
- Email addresses are unique across all users

---

## Features

- **Store Planner** — Create stores, define aisle layouts (numbered, lettered, or custom), and assign products to aisles
- **Shopping Lists** — One active list per store; add products by browsing aisles or searching; update quantities; remove items
- **Past Lists** — Closed lists are preserved; you can start a new list from a past one (overwrite or merge)
- **Standalone Products** — A global product library that can be linked to multiple stores, each with their own aisle assignment
- **Session Auth** — HTTP-only cookie sessions, 7-day expiry, constant-time password comparison

---

## Notes

- All users share equal permissions — any logged-in user can edit any store, product, or shopping list
- The app is designed for private/home use on a local network or self-hosted server
- Sessions expire after 7 days; logging out immediately invalidates the session token