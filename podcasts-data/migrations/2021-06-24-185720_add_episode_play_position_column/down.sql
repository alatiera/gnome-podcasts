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
        podcast_id      INTEGER NOT NULL,
        favorite        INTEGER DEFAULT 0,
        archive INTEGER DEFAULT 0,
        PRIMARY KEY (title, podcast_id)
);

INSERT INTO episodes (title, uri, local_uri, description, epoch, length, duration, guid, played, favorite, archive, podcast_id)
SELECT title, uri, local_uri, description, epoch, length, duration, guid, played, favorite, archive, podcast_id
FROM old_table;
Drop table old_table;
