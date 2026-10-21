-- Enrollment-based access, saved places, school changes, review school snapshots and edit tracking.

-- Month the student started at their current school (always the 1st). Access lasts 5 years from it.
ALTER TABLE users ADD COLUMN enrolled_on DATE;
UPDATE users SET enrolled_on = date_trunc('month', created_at)::date;
ALTER TABLE users ALTER COLUMN enrolled_on SET NOT NULL;
ALTER TABLE users ADD CONSTRAINT users_enrolled_on_first_of_month CHECK (extract(day FROM enrolled_on) = 1);

CREATE TABLE saved_places (
    user_id     UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    place_id    UUID NOT NULL REFERENCES places (id) ON DELETE CASCADE,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
    PRIMARY KEY (user_id, place_id)
);
CREATE INDEX saved_places_user_recent_idx ON saved_places (user_id, created_at DESC);

-- Reviews keep the school the author had when posting, even after a school change.
ALTER TABLE reviews ADD COLUMN school_id UUID REFERENCES schools (id);
UPDATE reviews r SET school_id = u.school_id FROM users u WHERE u.id = r.user_id;
-- Set only when the author changes rating or body.
ALTER TABLE reviews ADD COLUMN edited_at TIMESTAMPTZ;

-- At most one school change in progress per user. The code columns follow verification_codes' rules.
CREATE TABLE school_changes (
    user_id       UUID PRIMARY KEY REFERENCES users (id) ON DELETE CASCADE,
    email         TEXT NOT NULL CHECK (email = lower(email)),
    email_domain  TEXT NOT NULL,
    school_id     UUID REFERENCES schools (id),
    enrolled_on   DATE NOT NULL CHECK (extract(day FROM enrolled_on) = 1),
    code_hash     TEXT NOT NULL,
    expires_at    TIMESTAMPTZ NOT NULL,
    attempts      INT NOT NULL DEFAULT 0,
    code_sent_at  TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX school_changes_domain_idx ON school_changes (email_domain);
