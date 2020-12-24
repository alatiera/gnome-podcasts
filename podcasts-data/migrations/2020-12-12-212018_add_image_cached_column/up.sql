ALTER TABLE shows
    RENAME TO old_table;

CREATE TABLE shows
(
    `id`           INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT UNIQUE,
    `title`        TEXT    NOT NULL,
    `link`         TEXT    NOT NULL,
    `description`  TEXT    NOT NULL,
    `image_uri`    TEXT,
    `image_cached` TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    `source_id`    INTEGER NOT NULL UNIQUE
);

INSERT INTO shows (id, title, link, description, image_uri, source_id)
SELECT id, title, link, description, image_uri, source_id
FROM old_table;
Drop table old_table;
