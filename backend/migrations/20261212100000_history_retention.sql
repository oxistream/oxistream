-- Days the watch history is kept; NULL keeps it forever. The operator maximum still applies.
ALTER TABLE user_settings
    ADD COLUMN history_retention_days INTEGER
        CHECK (history_retention_days IN (1, 7, 14, 30, 90, 180, 365));

-- Retention cleanup scans by age.
CREATE INDEX watch_progress_updated_idx ON watch_progress (updated_at);
