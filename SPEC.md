# Filament Inventory Tracker — Final Spec

## 1. Purpose

Personal, self-hosted app to track 3D printer filament stock: what spools exist,
their brand/material/colour, product links, photos, notes, and roughly how much
filament is left (via weight-based calculation). Single user. Solves "I have to
open a bunch of boxes to know what filament I have."

## 2. Stack decisions (with rationale, so Claude Code doesn't need to re-derive them)

| Decision | Choice | Why |
|---|---|---|
| Language | Rust | User preference — modern, safe, fast |
| Web framework | Axum | Tokio-team maintained, Tower middleware ecosystem, current default recommendation for new Rust web projects in 2026 |
| Templates | **Tera** (not Askama) | Runtime template loading = instant refresh during dev (edit HTML, reload browser, no rebuild). Askama requires a full recompile on every template change, which conflicts with the explicit priority on fast local iteration. Trade-off accepted: template variable typos become runtime errors instead of compile-time errors — acceptable for a single-user hobby app. |
| Interactivity | htmx | SPA-like interactivity (filtering, inline updates, modals) without a JS build pipeline or a separate frontend codebase |
| Database | SQLite via SQLx | Single-user, zero-config, single-file backups, no second container needed |
| Image storage | Filesystem, path stored in DB | Standard approach; doesn't bloat the DB; same volume as the DB file so one backup covers both |
| Auth | **None in-app** | Cloudflare Tunnel + Cloudflare Access (email allowlist) sits in front and handles identity. The app must bind only to the container's internal interface — never expose its port publicly outside the tunnel. |
| Deployment | Single Docker container, docker-compose, QNAP Container Station | No second DB container needed (SQLite is embedded); one persistent volume for `/data` (db + images) |

## 3. Mobile requirements

- Fully responsive, mobile-first CSS — same server-rendered HTML serves both desktop and mobile, no separate app/codebase.
- Spool list: table layout on desktop (`min-width: 768px` breakpoint), stacked cards on mobile (image thumbnail, brand/material/colour, remaining % as a progress bar).
- Touch targets ≥ ~44px; generous spacing on filter controls and sortable column headers.
- Image upload input: `<input type="file" accept="image/*" capture="environment">` so mobile browsers offer "take photo" directly.
- Required: `<meta name="viewport" content="width=device-width, initial-scale=1">`.
- No CSS framework required — plain stylesheet, or Tailwind via CDN if utility classes are preferred. Either is fine at this app's scale.

## 4. Data model

```sql
CREATE TABLE brands (
  id INTEGER PRIMARY KEY,
  name TEXT NOT NULL,
  tare_weight_g REAL NOT NULL,   -- default/typical empty-spool weight for this brand
  notes TEXT
);

CREATE TABLE materials (
  id INTEGER PRIMARY KEY,
  name TEXT NOT NULL,            -- PLA, PETG, ABS, TPU, ASA, etc.
  notes TEXT
);

CREATE TABLE colours (
  id INTEGER PRIMARY KEY,
  name TEXT NOT NULL,
  hex TEXT                       -- for swatch display, e.g. "#FF5733"
);

CREATE TABLE spools (
  id INTEGER PRIMARY KEY,
  brand_id INTEGER NOT NULL REFERENCES brands(id),
  material_id INTEGER NOT NULL REFERENCES materials(id),
  colour_id INTEGER NOT NULL REFERENCES colours(id),
  nominal_weight_g REAL NOT NULL DEFAULT 1000,  -- label weight: 1000/750/500/250 etc.
  tare_override_g REAL,          -- nullable; falls back to brands.tare_weight_g if null
  product_url TEXT,
  notes TEXT,
  purchase_date TEXT             -- ISO date
);

CREATE TABLE spool_images (
  id INTEGER PRIMARY KEY,
  spool_id INTEGER NOT NULL REFERENCES spools(id) ON DELETE CASCADE,
  path TEXT NOT NULL,
  sort_order INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE weight_log (
  id INTEGER PRIMARY KEY,
  spool_id INTEGER NOT NULL REFERENCES spools(id) ON DELETE CASCADE,
  gross_weight_g REAL NOT NULL,  -- spool + remaining filament, as measured
  measured_at TEXT NOT NULL      -- ISO datetime
);
```

**Derived values (computed, not stored):**
```
remaining_g        = latest(weight_log.gross_weight_g for spool) - (tare_override_g or brands.tare_weight_g)
percent_remaining  = remaining_g / nominal_weight_g
```
Compute via a JOIN against the latest `weight_log` row per spool (subquery or SQL view). Keeps values always consistent and gives free historical tracking per spool (e.g. a usage graph later, if wanted).

## 5. Pages / routes

| Route | Purpose |
|---|---|
| `GET /spools` | Main inventory list — filterable, sortable |
| `GET /spools/:id` | Spool detail: images, weight history, notes, product link |
| `GET /spools/new`, `POST /spools` | Add-spool wizard |
| `POST /spools/:id/weigh` | Log a new gross weight reading |
| `GET /brands`, `POST /brands`, `PUT /brands/:id`, `DELETE /brands/:id` | Brands tab — editable reference list |
| `GET /materials`, `POST /materials`, `PUT /materials/:id`, `DELETE /materials/:id` | Materials tab — same shape as brands |
| `GET /colours`, `POST /colours`, `PUT /colours/:id`, `DELETE /colours/:id` | Colours tab — same shape as brands |

Brands/Materials/Colours share **one generic "editable reference table" Tera partial/macro** — identical CRUD shape, different table/fields. Build once, reuse three times. Navigation: `Spools | Add Spool | Brands | Materials | Colours` as top-level tabs, structured so more tabs can be added later without rework.

## 6. Add-spool wizard (steps)

1. **Brand** — dropdown, populated from the Brands tab. No inline "add brand" here — brands are managed only via the Brands tab.
2. **Material** — dropdown, from the Materials tab.
3. **Colour** — dropdown, from the Colours tab; show the hex swatch next to each option.
4. **Weight** — toggle:
   - "Brand new / unopened" → app computes `gross = tare + nominal` automatically (nominal defaults to 1000g, editable).
   - "Already opened" → user enters the current gross weight from a scale.
   - Either path creates the first `weight_log` row for this spool.
5. **Pictures** — multipart upload (one or more), saved to `IMAGE_DIR`, paths stored in `spool_images`.
6. **Product link + notes** — optional fields.

If a dropdown (brand/material/colour) is empty, don't show a dead-end empty `<select>` — show a message linking to that tab to add one first.

## 7. Filtering & sorting (main spool list)

- Query params: `?material=&brand=&colour=&sort=&dir=` — all optional.
- Server builds the SQL `WHERE` clause conditionally based on which params are present.
- Every column header is clickable (`hx-get`), toggling `dir=asc|desc` on repeat clicks of the same column; show ▲/▼ on the active sort column.
- **Whitelist `sort` against a fixed allowed set** (`brand`, `material`, `colour`, `remaining_g`, `percent`, `purchase_date`) before use in `ORDER BY` — never interpolate the raw query param into SQL.
- The endpoint returns just the `<tbody>` / card-list fragment for htmx to swap in. Filter and sort state live in the URL — bookmarkable, shareable, no client-side state management needed.

## 8. Local development workflow (no Docker — primary dev loop)

- **Auto-rebuild/restart:** `cargo install cargo-watch`, then `cargo watch -x run`. Rust changes trigger rebuild + restart. Template changes (Tera) need no rebuild at all — just refresh the browser.
- **Local SQLite:** `DATABASE_URL` points to `./data/dev.db` (gitignored).
- **Local images folder:** `./data/images/` (gitignored).
- **Migrations:** use `sqlx-cli` (`cargo install sqlx-cli`); `sqlx migrate run` applies schema migrations — same migration files run identically in Docker and on QNAP.
- **Config:** load `DATABASE_URL`, `IMAGE_DIR`, `PORT` etc. from a `.env` file via the `dotenvy` crate. Same variable names get set via `docker-compose.yml` later, so the binary doesn't need to know which environment it's running in.
- **Testing on phone during dev:** bind to `0.0.0.0` and hit `http://<laptop-LAN-IP>:PORT` from a phone browser on the same network, to check mobile CSS live.

## 9. Docker (for parity checks before deploying, and for QNAP)

Single `docker-compose.yml`, usable both locally and on QNAP — only the host volume path differs:

```yaml
services:
  filament-tracker:
    build: .
    ports:
      - "8080:8080"
    volumes:
      - ./data:/data
    environment:
      - DATABASE_URL=sqlite:///data/db.sqlite
      - IMAGE_DIR=/data/images
```

- Multi-stage Dockerfile: `rust:slim` builder → minimal runtime (`debian:slim` or distroless) for a small final image.
- Check QNAP CPU architecture before building (most modern models are x86_64; some are ARM) — use `docker buildx` for multi-arch if needed.
- One persistent volume covering both the SQLite file and the images folder, so a single backup job covers everything.
- Run `docker compose up --build` occasionally to confirm the containerized build works before pushing to QNAP — this is a parity check, not the everyday dev loop (section 8 is faster for iteration).
- No auth/session middleware in the app itself — Cloudflare Access is the identity layer; bind the app to the container's internal interface only.

## 10. Explicitly out of scope (investigated, deliberately not building)

- **Bambu Lab printer API integration** — investigated for the user's Bambu P2S. Bambu's MQTT telemetry does not expose a reliable "grams consumed" figure (only a rotation-based estimate for the AMS's own tracking), so automated consumption deduction isn't worth building. Manual/scale-based weight logging (section 4/6) is the sole mechanism, by design.
- **Multi-user accounts / permissions** — single user; Cloudflare Access handles access gating upstream.

## 11. Suggested build order

1. Project scaffold: Cargo project, Axum server boot, `.env`/`dotenvy` config loading, SQLx + SQLite migrations for the schema in section 4.
2. Brands / Materials / Colours tabs (generic CRUD Tera partial) — needed first since the wizard depends on them.
3. Add-spool wizard (section 6).
4. Spool list page with filter + sort (section 7, htmx fragment swapping).
5. Spool detail page + weight-log entry form + remaining-grams/percent calculation (section 4).
6. Image upload handling + mobile-responsive CSS pass (section 3).
7. Dockerfile + docker-compose + local Docker parity test (section 9).
8. Deploy to QNAP Container Station behind the existing Cloudflare Tunnel.

Work through these as separate, verifiable steps rather than attempting the whole app at once — run `cargo build`/`cargo test` between steps to catch issues before they compound.
