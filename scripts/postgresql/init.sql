CREATE TABLE IF NOT EXISTS commits (
    stream_id      UUID NOT NULL,
    stream_version INTEGER NOT NULL,
    commit_id         UUID NOT NULL,
    commit_sequence   INTEGER NOT NULL,
    commit_number     SERIAL PRIMARY KEY,
    commit_timestamp  TIMESTAMP WITH TIME ZONE NOT NULL,
    events_count      INTEGER NOT NULL,
    metadata          JSON NOT NULL,
    events            JSON NOT NULL,
    dispatched        BOOLEAN NOT NULL DEFAULT FALSE
);
CREATE UNIQUE INDEX IF NOT EXISTS commits_commit_id_unique_idx ON commits (commit_id);
CREATE UNIQUE INDEX IF NOT EXISTS commits_commit_stream_idx ON commits (stream_id, stream_version);
CREATE UNIQUE INDEX IF NOT EXISTS commits_commit_sequence_idx ON commits (stream_id, commit_sequence);
CREATE INDEX IF NOT EXISTS commits_dispatched_idx ON commits (dispatched);