ALTER TABLE episode RENAME TO old_table;

CREATE TABLE episode (
    id	INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT UNIQUE,
    title	TEXT,
    uri	TEXT NOT NULL UNIQUE,
    local_uri	TEXT,
    description	TEXT,
    published_date	TEXT,
    epoch	INTEGER NOT NULL DEFAULT 0,
    length	INTEGER,
    guid	TEXT,
    played	INTEGER,
    favorite	INTEGER NOT NULL DEFAULT 0,
    archive	INTEGER NOT NULL DEFAULT 0,
    podcast_id	INTEGER NOT NULL
);
 
INSERT INTO episode SELECT * FROM old_table;