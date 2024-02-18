ALTER TABLE episodes RENAME TO old_table;

CREATE TABLE episodes (
        title   TEXT NOT NULL,
        uri     TEXT,
        local_uri       TEXT,
        description     TEXT,
        epoch   INTEGER NOT NULL DEFAULT 0,
        length  INTEGER,
        duration        INTEGER,
        guid    TEXT,
        played  INTEGER,
        play_position  INTEGER NOT NULL,
        show_id      INTEGER NOT NULL,
        PRIMARY KEY (title, show_id)
);

INSERT INTO episodes (title, uri, local_uri, description, epoch, length, duration, guid, played, show_id, play_position)
SELECT title, uri, local_uri, description, epoch, length, duration, guid, played, show_id, play_position
FROM old_table;
Drop table old_table;
