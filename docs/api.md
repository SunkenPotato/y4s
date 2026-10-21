# Corner API contract (v1)

This is the only interface between `api/` (Rust/axum) and `web/` (Next.js). Changing it requires the coordinator to edit this file first.

- **Base URL (dev):** `http://localhost:4000`. All routes are under `/api`.
- **Browser calls:** the browser calls `/api/*` on the web origin (`localhost:3000`). Next.js rewrites those to the API, so the session cookie is first-party.
- **Server components:** they call `API_URL` (`http://localhost:4000`) directly and forward the incoming `cookie` header.

## Conventions

- **Format:** JSON in and out, `snake_case` keys, timestamps as RFC 3339 UTC strings, IDs as UUID strings.
- **Auth:** an opaque session token in the `corner_session` cookie.
  - Set as `HttpOnly; SameSite=Lax; Path=/; Max-Age=2592000` (30 days), plus `Secure` when `COOKIE_SECURE=true`.
  - The server stores only the SHA-256 hash of the token.
- **CSRF:** every non-GET request must carry `Content-Type: application/json` (an empty `{}` body is fine). When an `Origin` header is present, it must be in `ALLOWED_ORIGINS`. Requests that fail either check get `403 forbidden_origin`.
- **Pagination:** list endpoints accept `limit` (default 20, max 50) and `cursor`, and return `{ "items": [...], "next_cursor": "opaque" | null }`.
- **Errors:** a non-2xx response always has this shape:
  ```json
  { "error": { "code": "snake_case_code", "message": "Human sentence safe to show.", "fields": { "email": "…" } } }
  ```
  `fields` is present only on `422 validation`. Messages follow DESIGN.md › Writing: say what's wrong and how to fix it.
- **Common codes:**

  | Code | Status | Meaning |
  | --- | --- | --- |
  | `unauthenticated` | 401 | No valid session |
  | `forbidden` | 403 | Signed in, but the action isn't allowed |
  | `not_verified` | 403 | Signed in, but status isn't `verified` |
  | `not_found` | 404 | |
  | `validation` | 422 | Includes `fields` |
  | `rate_limited` | 429 | Includes a `Retry-After` header and `retry_after` seconds in the error |
  | `internal` | 500 | |

## Shared objects

```jsonc
// SchoolSummary
{ "id": "uuid", "name": "UCLA", "kind": "university" | "high_school", "city": "Los Angeles", "state": "CA" }

// School = SchoolSummary + { "lat": 34.0689, "lng": -118.4452 }

// Me (only ever returned to the user themself)
{
  "id": "uuid",
  "email": "maya.r@g.ucla.edu",
  "first_name": "Maya",
  "last_name": "Rivera",
  "display_name": "Maya R.",          // first name + last initial, derived by the server
  "role": "user" | "admin",
  "status": "pending_email" | "pending_school" | "verified" | "rejected" | "suspended",
  "school": SchoolSummary | null,     // null until the domain is known or a school is chosen
  "needs_school": true | false,       // the email domain is unknown and no school has been chosen yet
  "created_at": "…"
}

// Author (public)
{ "display_name": "Maya R.", "school_name": "UCLA", "school_kind": "university" }

// Place (list item)
{
  "id": "uuid",
  "kind": "business" | "facility",     // facilities are on campus and always have school_id
  "name": "Gayley Noodle Bar",
  "category": "food" | "coffee" | "study" | "gym" | "services" | "nightlife" | "groceries" | "other",
  "subcategory": "Ramen" | null,       // free text, max 40 chars
  "price_level": 1 | 2 | 3 | 4 | null, // rendered as $ … $$$$
  "address": "1062 Gayley Ave, Los Angeles, CA 90024",
  "lat": 34.0632, "lng": -118.4469,
  "school_id": "uuid" | null,
  "student_discount": "15% off Mon–Thu" | null,  // max 80 chars
  "rating_avg": 4.6 | null,            // 1 decimal; null when there are no reviews
  "review_count": 212,
  "distance_mi": 0.3 | null,           // only when the request has near=
  "top_quote": { "text": "The spicy miso is worth the line.", "author": Author, "review_id": "uuid" } | null,
  "status": "active" | "pending" | "hidden"
}

// PlaceDetail = Place + {
//   "hours": [ { "days": "Mon – Thu", "time": "11 am – 10 pm" } ] | null,
//   "rating_distribution": { "1": 5, "2": 8, "3": 19, "4": 52, "5": 128 },
//   "school": SchoolSummary | null,
//   "viewer_review_id": "uuid" | null    // the viewer's own review of this place, if any
// }

// Review
{
  "id": "uuid",
  "place_id": "uuid",
  "rating": 1-5,
  "body": "…",                        // 50–2000 chars
  "visited_month": "2026-10" | null,
  "used_deal": true | false,
  "helpful_count": 24,
  "viewer_voted": true | false,
  "is_mine": true | false,
  "author": Author,
  "created_at": "…", "updated_at": "…"
}
```

`top_quote.text` is the first sentence of the place's most helpful review from the last 90 days, at most 140 chars, ending at a sentence boundary when possible.

## Auth and verification

The flow is **signup → (choose school, if `needs_school`) → enter code → `verified`**, or `pending_school` when the domain is waiting for admin approval.

### POST /api/auth/signup
```json
{ "email": "maya.r@g.ucla.edu", "password": "…", "first_name": "Maya", "last_name": "Rivera", "birth_year": 2005 }
```
On success: **201** `{ "user": Me }`, the session cookie is set (status `pending_email`), and a 6-digit code is emailed immediately.

The email domain is checked in this order:
1. **Personal providers** (gmail.com, googlemail.com, yahoo.\*, outlook.com, hotmail.\*, live.com, icloud.com, me.com, aol.com, proton.me, protonmail.com, gmx.\*, mail.com, yandex.\*, zoho.com) → **422** `not_school_email`.
2. **Approved** in `school_domains` → the user is linked to that school, and `needs_school` is false.
3. **Ends in `.edu`** (or a subdomain of a `.edu`) but unknown → `needs_school: true`. Picking a school auto-approves the domain.
4. **Anything else** → `needs_school: true`. Picking a school creates a **pending** domain request.

Errors:

| Code | Status | Condition |
| --- | --- | --- |
| `validation` | 422 | Fields: email, password (min 10 chars), first_name / last_name (1–40), birth_year |
| `not_school_email` | 422 | Personal email provider |
| `too_young` | 422 | Age under 13, computed from birth_year as of the current year |
| `email_taken` | 409 | |
| `rate_limited` | 429 | |

### POST /api/auth/school (session required, `needs_school` must be true)
```json
{ "school_id": "uuid" }
// or
{ "new_school": { "name": "University High School", "kind": "high_school", "city": "Los Angeles", "state": "CA" } }
```
Sets `user.school` and links the email domain to that school: approved for `.edu`, pending otherwise. If another user's pending request for the same domain names a different school, the existing request wins and this user is linked to it.

Returns **200** `{ "user": Me }`. Errors: `forbidden` (the school was already set), `validation`, `not_found`.

### POST /api/auth/verify
```json
{ "code": "482913" }
```
Codes are stored hashed and expire after 15 minutes. Each code allows 5 attempts; after that the user must request a new one.

Returns **200** `{ "user": Me }`. The status becomes:
- `verified` if the domain is approved,
- `pending_school` if the domain is pending or `needs_school` is still true,
- `rejected` if the domain was rejected.

Verifying sets `verified_at`, which is used for yearly re-verification later. If the email is in the `ADMIN_EMAILS` env var, the role becomes `admin` on verification.

Errors:

| Code | Status | Details |
| --- | --- | --- |
| `invalid_code` | 422 | Includes `"attempts_left": n` |
| `code_expired` | 422 | |
| `too_many_attempts` | 422 | |
| `already_verified` | 409 | |

### POST /api/auth/resend
**204**. Sends a new code and invalidates the old one. There is a 60s cooldown per user; during it the response is **429** `rate_limited` with `retry_after`.

### POST /api/auth/login
```json
{ "email": "…", "password": "…" }
```
**200** `{ "user": Me }` and sets the cookie. Errors: **401** `invalid_credentials` (the same message for an unknown email and a wrong password), **403** `suspended`.

### POST /api/auth/logout
**204**. Deletes the session and clears the cookie.

### GET /api/me
**200** `{ "user": Me }` or **401**.

### PATCH /api/me
```json
{ "first_name": "…", "last_name": "…" }
```
**200** `{ "user": Me }`.

## Schools

### GET /api/schools?q=univ&kind=high_school&limit=10
**200** `{ "items": [School] }`. Matching is case-insensitive on the name prefix and words. No auth needed.

## Feed and places

### GET /api/feed?school_id=uuid
School defaults to the viewer's school, then to the first seeded school.

**200**:
```json
{
  "school": School,
  "featured": { "place": Place, "quote": { "text": "…", "author": Author, "review_id": "uuid" } } | null,
  "popular": [Place],   // up to 8, most reviews in the last 30 days, then by rating
  "deals": [Place]      // up to 6 that have a student_discount
}
```

### GET /api/places
Query parameters:

| Parameter | Values / meaning |
| --- | --- |
| `q` | Full-text search over name, subcategory and address |
| `category` | One of the category enum values |
| `kind` | `business` or `facility` |
| `school_id` | Facilities of that school, plus businesses within 2 mi of it |
| `near` | `lat,lng` |
| `radius_mi` | Default 1, max 10 |
| `deals` | `true` |
| `sort` | `reviews` (default) \| `rating` \| `distance` (requires `near`) \| `new` |
| `cursor`, `limit` | See Pagination |

**200** a paginated list of `Place`. Only `active` places are returned.

### GET /api/places/:id
**200** `{ "place": PlaceDetail }`. A `pending` place is visible only to its creator and admins; everyone else gets 404.

### POST /api/places (verified)
```json
{ "kind": "business", "name": "…", "category": "food", "subcategory": "Ramen", "address": "…",
  "lat": 34.06, "lng": -118.44, "school_id": "uuid" | null, "price_level": 1, "student_discount": "…", "hours": [ … ] }
```
`lat`/`lng` are optional. When they're missing, the server geocodes the address with OSM Nominatim. The geocoder is behind a trait and stubbed in tests.

**201** `{ "place": PlaceDetail }` with status `pending`; an admin approves it.

Errors:

| Code | Status | Details |
| --- | --- | --- |
| `validation` | 422 | |
| `geocode_failed` | 422 | The `address` field message reads "We couldn't find this address. Check the street and city." |
| `duplicate_place` | 409 | Same normalized name within 50 m; includes `"existing_id"` |

### GET /api/places/:id/reviews?sort=helpful|new|lowest&cursor=&limit=
**200** a paginated list of `Review`. Default sort is `helpful` (helpful_count desc, then newest).

### POST /api/places/:id/reviews (verified)
```json
{ "rating": 4, "body": "…", "visited_month": "2026-10", "used_deal": true }
```
**201** `{ "review": Review }`.

Errors:

| Code | Status | Details |
| --- | --- | --- |
| `validation` | 422 | Messages: rating "Pick a rating from 1 to 5 stars."; body "Write at least 50 characters so it's useful." |
| `already_reviewed` | 409 | Includes `"review_id"` |
| `not_verified` | 403 | |

### PATCH /api/reviews/:id (author only)
Accepts any subset of the POST fields. **200** `{ "review": Review }`.

### DELETE /api/reviews/:id (author or admin)
**204**. A soft delete (`deleted_at`). Within 5 minutes the author can call `POST /api/reviews/:id/restore` (**200** `{ "review": Review }`); this backs the Undo toast.

### PUT /api/reviews/:id/vote (verified, not the author) and DELETE /api/reviews/:id/vote
Both are idempotent. **200** `{ "helpful_count": 25, "viewer_voted": true|false }`.

### POST /api/reports (verified)
```json
{ "target_type": "review" | "place", "target_id": "uuid", "reason": "spam" | "offensive" | "not_a_student" | "wrong_info" | "other", "details": "…" }
```
**201** `{ "id": "uuid" }`. A duplicate report from the same user on the same target returns 200 with the existing id.

## Admin (role `admin`, otherwise 403)

### GET /api/admin/domains?status=pending|approved|rejected
**200**:
```json
{ "items": [ {
  "domain": "students.lausd.net",
  "status": "pending",
  "school": SchoolSummary,
  "waiting_count": 4,
  "requested_at": "…",
  "decided_at": null,
  "flags": ["personal_provider"]
} ] }
```
`waiting_count` is the number of users with status `pending_school` or `rejected` on this domain. `flags` is empty in normal cases.

### Domain decisions
- **POST /api/admin/domains/:domain/approve** → **200** with the domain item. Every `pending_school` or `rejected` user on that domain who has confirmed their email becomes `verified` and gets a "You're in" email.
- **POST /api/admin/domains/:domain/reject** `{ "reason": "…" }` → **200**. Waiting users become `rejected` and are emailed.
- **POST /api/admin/domains/:domain/reset** → **200**. Sets the domain back to pending (Undo) and sets affected users back to `pending_school`.

### Places
- **GET /api/admin/places?status=pending** → a paginated list of `PlaceDetail` plus `"created_by": Author`.
- **POST /api/admin/places/:id/approve** and **POST /api/admin/places/:id/hide** → **200** `{ "place": PlaceDetail }`.

### Reports
- **GET /api/admin/reports?status=open|resolved** → a paginated list of:
  ```json
  { "id", "target_type", "target_id", "target_preview": "first 140 chars or place name", "reason", "details", "reporter": Author, "report_count": 3, "status", "created_at" }
  ```
- **POST /api/admin/reports/:id/resolve** `{ "action": "dismiss" | "remove_content" }` → **200**. `remove_content` hides the place or soft-deletes the review, and resolves every open report on that target.

## Rate limits
- `/api/auth/*`: 10 requests per minute per IP.
- Resending a code: 1 per 60 s per user.
- Writes (reviews, places, reports, votes): 30 per minute per user.

## Environment (api)
```
DATABASE_URL=postgres://corner:corner@localhost:5433/corner
PORT=4000
ALLOWED_ORIGINS=http://localhost:3000
COOKIE_SECURE=false
MAIL_TRANSPORT=smtp            # smtp | resend | log
SMTP_HOST=localhost
SMTP_PORT=1025
MAIL_FROM="Corner <hello@corner.local>"
RESEND_API_KEY=                # only when MAIL_TRANSPORT=resend
ADMIN_EMAILS=admin@ucla.edu
NOMINATIM_URL=https://nominatim.openstreetmap.org
```

## Environment (web)
```
API_URL=http://localhost:4000      # server-side fetches and the rewrite target
```
