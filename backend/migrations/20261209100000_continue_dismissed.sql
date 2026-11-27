-- Hidden from "Continue watching" while the entry stays in the history; the next progress
-- report shows it again.
ALTER TABLE watch_progress ADD COLUMN continue_dismissed BOOLEAN NOT NULL DEFAULT false;
