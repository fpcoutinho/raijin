CREATE TYPE theme_preference AS ENUM ('light', 'dark', 'system');

ALTER TABLE users
    ADD COLUMN full_name          TEXT,
    ADD COLUMN professional_title TEXT,
    ADD COLUMN theme_preference   theme_preference NOT NULL DEFAULT 'system';
