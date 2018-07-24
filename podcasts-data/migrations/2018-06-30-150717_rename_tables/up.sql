ALTER TABLE episode RENAME TO old_table;
ALTER TABLE podcast RENAME TO shows;

CREATE TABLE episodes (
  title	TEXT NOT NULL,
  uri	TEXT,
  local_uri	TEXT,
  description	TEXT,
  epoch	INTEGER NOT NULL DEFAULT 0,
  length	INTEGER,
  duration	INTEGER,
  guid	TEXT,
  played	INTEGER,
  show_id	INTEGER NOT NULL,
  PRIMARY KEY (title, show_id)
);

INSERT INTO episodes (
  title,
  uri,
  local_uri,
  description,
  epoch,
  length,
  duration,
  guid,
  played,
  show_id
) SELECT title,
  uri,
  local_uri,
  description,
  epoch, length,
  duration,
  guid,
  played,
  podcast_id
FROM old_table;

Drop table old_table;
