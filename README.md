# hungry-family

A self-hosted collaborative meal planning and grocery shopping app for the family. Built with a Rust/Axum backend serving HTMX-driven HTML, and a PostgreSQL database.

The core problem it solves: keeping a shared, up-to-date shopping list so you stop defaulting to takeout. Any family member can log in, browse products organized by store aisle, and add items to the active shopping list.

---

## Tech Stack

| Layer | Technology |
|---|---|
| Frontend | HTMX (vendored, no build step) |
| Templating | Askama (Rust, compile-time checked) |
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
│   └── bin/
│       └── hash_password.rs  # Helper binary to hash passwords
├── templates/              # Askama templates (layout + pages + fragments)
├── migrations/             # SQL migration files (run in order)
│   ├── 0001_init.sql
│   ├── 0002_store_layouts.sql
│   └── 0003_shopping_lists_v2.sql
├── static/                 # Hand-maintained assets (served by Axum)
│   ├── vendor/htmx.min.js  # Vendored htmx (see version manifest)
│   ├── vendor/htmx.version # Pinned htmx version (Renovate-tracked)
│   └── app.css
├── scripts/
│   └── vendor-htmx.sh      # Downloads the pinned htmx release
└── tests/
    └── htmx_vendor.rs      # Asserts vendored htmx matches the manifest
```

---

## Prerequisites

- **Rust** (stable, 2024 edition) — [rustup.rs](https://rustup.rs)
- **PostgreSQL** (v14+ recommended)

No Node.js or npm — the frontend has no build step.

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

### 4. Run the server

```bash
cargo run
```

The server starts on **http://localhost:800** (port 800 requires `cap_net_bind_service` on Linux — see `.cargo/config.toml` for the runner configuration that handles this automatically on `x86_64-unknown-linux-gnu`).

---

## HTMX dependency management

`htmx.min.js` is vendored into `static/vendor/` and pinned by version in `static/vendor/htmx.version`. [Renovate](https://docs.renovatebot.com/) (GitHub App) watches the manifest via a regex custom manager in `.github/renovate.json` and opens a PR whenever a new htmx release is published. When such a PR lands:

```bash
scripts/vendor-htmx.sh   # downloads the newly pinned version + prints its sha256
cargo test               # verifies the vendored file matches the manifest
```

Install the free Renovate GitHub App on the repository to activate update PRs.

---

## Development Workflow

```bash
cargo run
```

Edit templates in `templates/` and assets in `static/`, then refresh the browser. Askama templates are compile-time checked, so `cargo check` catches template errors.

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

## Status

The previous React frontend and its JSON API have been removed. The database schema, migrations, and auth foundation (session management, password verification, `CurrentAccount` extractor) remain in place. The HTMX-based UI is being rebuilt; only the skeleton page exists so far.

## Notes

- All users share equal permissions — any logged-in user can edit any store, product, or shopping list
- The app is designed for private/home use on a local network or self-hosted server
- Sessions expire after 7 days; logging out immediately invalidates the session token
