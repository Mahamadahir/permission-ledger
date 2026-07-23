CREATE EXTENSION IF NOT EXISTS pgcrypto;

CREATE TABLE users (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    email text NOT NULL UNIQUE,
    password_hash text NOT NULL,
    display_name text,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE user_settings (
    user_id uuid PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    timezone text NOT NULL DEFAULT 'UTC',
    review_reminder_days integer NOT NULL DEFAULT 30,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE sessions (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash text NOT NULL UNIQUE,
    csrf_token text NOT NULL,
    user_agent text,
    ip_address inet,
    expires_at timestamptz NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE login_rate_limits (
    email text PRIMARY KEY,
    failed_attempts integer NOT NULL DEFAULT 0,
    locked_until timestamptz,
    updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE services (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    normalized_domain text NOT NULL UNIQUE,
    name text NOT NULL,
    website_url text NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE consent_categories (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    slug text NOT NULL UNIQUE,
    name text NOT NULL,
    sort_order integer NOT NULL DEFAULT 0
);

CREATE TABLE consent_records (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    service_id uuid NOT NULL REFERENCES services(id) ON DELETE RESTRICT,
    category_id uuid NOT NULL REFERENCES consent_categories(id) ON DELETE RESTRICT,
    consent_type text NOT NULL,
    date_given date NOT NULL,
    review_date date,
    expiry_date date,
    status text NOT NULL CHECK (status IN ('active', 'needs_review', 'expired', 'revoked')),
    risk_level text NOT NULL CHECK (risk_level IN ('low', 'medium', 'high')),
    source text NOT NULL CHECK (source IN ('manual', 'extension')),
    notes text,
    revoked_at timestamptz,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX consent_records_user_status_idx ON consent_records(user_id, status);
CREATE INDEX consent_records_user_review_date_idx ON consent_records(user_id, review_date);
CREATE INDEX consent_records_user_created_at_idx ON consent_records(user_id, created_at DESC);

CREATE TABLE extension_devices (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name text NOT NULL,
    token_hash text NOT NULL UNIQUE,
    last_used_at timestamptz,
    revoked_at timestamptz,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE audit_logs (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id uuid REFERENCES users(id) ON DELETE SET NULL,
    event_type text NOT NULL,
    entity_type text,
    entity_id uuid,
    metadata jsonb NOT NULL DEFAULT '{}'::jsonb,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX audit_logs_user_created_at_idx ON audit_logs(user_id, created_at DESC);

CREATE TABLE export_jobs (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    format text NOT NULL CHECK (format IN ('json', 'csv')),
    created_at timestamptz NOT NULL DEFAULT now()
);

INSERT INTO consent_categories (slug, name, sort_order) VALUES
    ('cookies', 'Cookies', 10),
    ('marketing', 'Marketing', 20),
    ('data-sharing', 'Data sharing', 30),
    ('app-permissions', 'App permissions', 40),
    ('oauth-permissions', 'OAuth permissions', 50),
    ('privacy-policy', 'Privacy policy', 60),
    ('subscription-terms', 'Subscription terms', 70),
    ('financial-services', 'Financial services', 80),
    ('health-data', 'Health data', 90),
    ('education-platforms', 'Education platforms', 100),
    ('employment-platforms', 'Employment platforms', 110),
    ('social-media', 'Social media', 120),
    ('other', 'Other', 130)
ON CONFLICT (slug) DO NOTHING;
