ALTER TABLE episodes RENAME TO old_table;
ALTER TABLE shows RENAME TO podcast;

CREATE TABLE episode (
  title	TEXT NOT NULL,
  uri	TEXT,
  local_uri	TEXT,
  description	TEXT,
  epoch	INTEGER NOT NULL DEFAULT 0,
  length	INTEGER,
  duration	INTEGER,
  guid	TEXT,
  played	INTEGER,
  podcast_id	INTEGER NOT NULL,
  PRIMARY KEY (title, podcast_id)
);

INSERT INTO episode (
  title,
  uri,
  local_uri,
  description,
  epoch,
  length,
  duration,
  guid,
  played,
  podcast_id
) SELECT title,
  uri,
  local_uri,
  description,
  epoch, length,
  duration,
  guid,
  played,
  show_id
FROM old_table;

Drop table old_table;
