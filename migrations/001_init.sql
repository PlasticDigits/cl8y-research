CREATE EXTENSION IF NOT EXISTS vector;

CREATE TABLE IF NOT EXISTS documents (
    id uuid PRIMARY KEY,
    source_kind text NOT NULL,
    source_id text NOT NULL,
    collected_at timestamptz NOT NULL,
    citation text NOT NULL,
    body text NOT NULL,
    embedding vector(64) NOT NULL,
    metadata jsonb NOT NULL DEFAULT '{}'::jsonb,
    UNIQUE (source_kind, source_id)
);
