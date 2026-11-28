-- When the user last opened the channel's page; later uploads mark the channel as new.
-- Existing subscriptions start at the migration time.
ALTER TABLE subscriptions ADD COLUMN last_visited_at TIMESTAMPTZ NOT NULL DEFAULT now();
