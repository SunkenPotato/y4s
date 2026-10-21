# Corner

Corner is a student-only guide to places near campus. Verified college and high-school students review local businesses and campus facilities.

| Path | What |
| --- | --- |
| `api/` | Rust + axum + sqlx (Postgres) |
| `web/` | Next.js (App Router, TypeScript, CSS Modules) |
| `docs/api.md` | API contract |
| `DESIGN.md`, `design/tokens.css` | Locked design system |

## Run locally

```sh
docker compose up -d                 # Postgres on :5433, Mailpit on :8025
(cd api && cargo run --bin seed)     # migrate and load demo data
(cd api && cargo run)                # API on :4000
(cd web && npm install && npm run dev)  # web on :3000
```

Verification emails arrive in Mailpit at http://localhost:8025.
