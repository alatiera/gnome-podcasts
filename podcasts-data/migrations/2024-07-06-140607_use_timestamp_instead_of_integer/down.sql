ALTER TABLE episodes RENAME TO old_table;

CREATE TABLE episodes (
        id      INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT UNIQUE,
        title   TEXT NOT NULL,
        uri     TEXT,
        local_uri       TEXT,
        description     TEXT,
        image_uri     TEXT,
        epoch   INTEGER NOT NULL DEFAULT ,
        length  INTEGER,
        duration        INTEGER,
        guid    TEXT,
        played  INTEGER,
        play_position  INTEGER NOT NULL,
        show_id      INTEGER NOT NULL
);

INSERT INTO episodes (id, title, uri, local_uri, description, image_uri, (cast(strftime('%s', epoch) as int)), length, duration, guid, (cast(strftime('%s', played) as int)), show_id, play_position)
SELECT id, title, uri, local_uri, description, image_uri, epoch, length, duration, guid, played, show_id, play_position
FROM old_table;
Drop table old_table;
