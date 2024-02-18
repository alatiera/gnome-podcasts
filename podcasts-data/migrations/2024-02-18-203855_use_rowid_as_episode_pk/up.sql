ALTER TABLE episodes RENAME TO old_table;

CREATE TABLE episodes (
        id      INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT UNIQUE,
        title   TEXT NOT NULL,
        uri     TEXT,
        local_uri       TEXT,
        description     TEXT,
        image_uri     TEXT,
        epoch   INTEGER NOT NULL DEFAULT 0,
        length  INTEGER,
        duration        INTEGER,
        guid    TEXT,
        played  INTEGER,
        play_position  INTEGER NOT NULL,
        show_id      INTEGER NOT NULL
);

INSERT INTO episodes (id, title, uri, local_uri, description, image_uri, epoch, length, duration, guid, played, play_position, show_id)
SELECT rowid, title, uri, local_uri, description, image_uri, epoch, length, duration, guid, played, play_position, show_id
FROM old_table;
Drop table old_table;
