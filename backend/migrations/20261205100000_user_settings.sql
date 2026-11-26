-- Playback preferences and home page layout; NULL means the default. `video_codecs` is ordered
-- by preference, empty for automatic selection. `home_sections` holds the enabled section keys
-- in order; the frontend ignores keys it does not know.
ALTER TABLE user_settings
    ADD COLUMN video_codecs    TEXT[]  NOT NULL DEFAULT '{}',
    ADD COLUMN max_quality     INTEGER CHECK (max_quality > 0),
    ADD COLUMN default_quality INTEGER CHECK (default_quality > 0),
    ADD COLUMN home_sections   JSONB   CHECK (jsonb_typeof(home_sections) = 'array');
