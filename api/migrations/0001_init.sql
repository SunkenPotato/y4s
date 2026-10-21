-- Corner schema v1. Enums are TEXT + CHECK so the runtime query API maps them as plain strings.

CREATE TABLE schools (
    id          UUID PRIMARY KEY,
    name        TEXT NOT NULL,
    kind        TEXT NOT NULL CHECK (kind IN ('university', 'high_school')),
    city        TEXT NOT NULL,
    state       TEXT NOT NULL,
    lat         DOUBLE PRECISION,
    lng         DOUBLE PRECISION,
    website     TEXT,
    -- clock_timestamp() so rows inserted in one transaction keep their insertion order ("first seeded school").
    created_at  TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp()
);
CREATE INDEX schools_name_lower_idx ON schools (lower(name));

CREATE TABLE users (
    id             UUID PRIMARY KEY,
    email          TEXT NOT NULL UNIQUE CHECK (email = lower(email)),
    -- The school_domains key this email maps to (for .edu this is the registrable domain, e.g. g.ucla.edu -> ucla.edu).
    email_domain   TEXT NOT NULL,
    password_hash  TEXT NOT NULL,
    first_name     TEXT NOT NULL,
    last_name      TEXT NOT NULL,
    birth_year     INT NOT NULL,
    role           TEXT NOT NULL DEFAULT 'user' CHECK (role IN ('user', 'admin')),
    status         TEXT NOT NULL DEFAULT 'pending_email'
                   CHECK (status IN ('pending_email', 'pending_school', 'verified', 'rejected', 'suspended')),
    school_id      UUID REFERENCES schools (id),
    verified_at    TIMESTAMPTZ,
    -- decided_at of the domain decision that last changed this user's status; lets "reset" undo exactly that decision.
    domain_decided_at TIMESTAMPTZ,
    created_at     TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX users_email_domain_idx ON users (email_domain);

CREATE TABLE school_domains (
    domain         TEXT PRIMARY KEY,
    school_id      UUID NOT NULL REFERENCES schools (id),
    status         TEXT NOT NULL CHECK (status IN ('pending', 'approved', 'rejected')),
    requested_by   UUID REFERENCES users (id) ON DELETE SET NULL,
    requested_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    decided_by     UUID REFERENCES users (id) ON DELETE SET NULL,
    decided_at     TIMESTAMPTZ,
    reject_reason  TEXT
);
CREATE INDEX school_domains_status_idx ON school_domains (status);

CREATE TABLE verification_codes (
    id          UUID PRIMARY KEY,
    user_id     UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    code_hash   TEXT NOT NULL,
    expires_at  TIMESTAMPTZ NOT NULL,
    attempts    INT NOT NULL DEFAULT 0,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp()
);
CREATE INDEX verification_codes_user_idx ON verification_codes (user_id, created_at DESC);

CREATE TABLE sessions (
    token_hash  TEXT PRIMARY KEY,
    user_id     UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    expires_at  TIMESTAMPTZ NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX sessions_user_idx ON sessions (user_id);

CREATE TABLE places (
    id                UUID PRIMARY KEY,
    kind              TEXT NOT NULL CHECK (kind IN ('business', 'facility')),
    name              TEXT NOT NULL,
    category          TEXT NOT NULL
                      CHECK (category IN ('food', 'coffee', 'study', 'gym', 'services', 'nightlife', 'groceries', 'other')),
    subcategory       TEXT,
    price_level       SMALLINT CHECK (price_level BETWEEN 1 AND 4),
    address           TEXT NOT NULL,
    lat               DOUBLE PRECISION NOT NULL,
    lng               DOUBLE PRECISION NOT NULL,
    school_id         UUID REFERENCES schools (id),
    student_discount  TEXT,
    hours             JSONB,
    status            TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('active', 'pending', 'hidden')),
    created_by        UUID REFERENCES users (id) ON DELETE SET NULL,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
    name_norm         TEXT GENERATED ALWAYS AS (regexp_replace(lower(name), '[^a-z0-9]', '', 'g')) STORED,
    search            TSVECTOR GENERATED ALWAYS AS (
                          to_tsvector('english', name || ' ' || coalesce(subcategory, '') || ' ' || address)
                      ) STORED,
    CHECK (kind <> 'facility' OR school_id IS NOT NULL)
);
CREATE INDEX places_search_idx ON places USING GIN (search);
CREATE INDEX places_status_idx ON places (status);
CREATE INDEX places_school_idx ON places (school_id);
CREATE INDEX places_name_norm_idx ON places (name_norm);

CREATE TABLE reviews (
    id             UUID PRIMARY KEY,
    place_id       UUID NOT NULL REFERENCES places (id) ON DELETE CASCADE,
    user_id        UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    rating         SMALLINT NOT NULL CHECK (rating BETWEEN 1 AND 5),
    body           TEXT NOT NULL,
    visited_month  DATE,
    used_deal      BOOLEAN NOT NULL DEFAULT false,
    created_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    deleted_at     TIMESTAMPTZ,
    deleted_by     UUID REFERENCES users (id) ON DELETE SET NULL
);
CREATE UNIQUE INDEX reviews_one_per_user_idx ON reviews (place_id, user_id) WHERE deleted_at IS NULL;
CREATE INDEX reviews_place_idx ON reviews (place_id) WHERE deleted_at IS NULL;
CREATE INDEX reviews_user_idx ON reviews (user_id);

CREATE TABLE review_votes (
    review_id   UUID NOT NULL REFERENCES reviews (id) ON DELETE CASCADE,
    user_id     UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (review_id, user_id)
);

CREATE TABLE reports (
    id           UUID PRIMARY KEY,
    reporter_id  UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    target_type  TEXT NOT NULL CHECK (target_type IN ('review', 'place')),
    target_id    UUID NOT NULL,
    reason       TEXT NOT NULL CHECK (reason IN ('spam', 'offensive', 'not_a_student', 'wrong_info', 'other')),
    details      TEXT,
    status       TEXT NOT NULL DEFAULT 'open' CHECK (status IN ('open', 'resolved')),
    resolution   TEXT CHECK (resolution IN ('dismiss', 'remove_content')),
    resolved_by  UUID REFERENCES users (id) ON DELETE SET NULL,
    resolved_at  TIMESTAMPTZ,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX reports_target_idx ON reports (target_type, target_id);
CREATE UNIQUE INDEX reports_open_dedupe_idx ON reports (reporter_id, target_type, target_id) WHERE status = 'open';
