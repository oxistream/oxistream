-- Preferred audio track: NULL for the original audio, `interface` for the interface language,
-- else a language code such as `de` or `pt-BR`.
ALTER TABLE user_settings
    ADD COLUMN audio_language TEXT
        CHECK (audio_language ~ '^([a-z]{2,3}(-[A-Za-z0-9]{2,8})?|interface)$');
