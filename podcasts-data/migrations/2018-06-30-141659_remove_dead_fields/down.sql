ALTER TABLE episode RENAME TO old_table;

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
  favorite	INTEGER DEFAULT 0,
  archive	INTEGER DEFAULT 0,
  PRIMARY KEY (title, podcast_id)
);

INSERT INTO episode (title, uri, local_uri, description, epoch, length, duration, guid, played, podcast_id, favorite, archive)
SELECT title, uri, local_uri, description, epoch, length, duration, guid, played, podcast_id, 0, 0
FROM old_table;

Drop table old_table;

ALTER TABLE podcast RENAME TO old_table;
CREATE TABLE `podcast` (
  `id`	INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT UNIQUE,
  `title`	TEXT NOT NULL,
  `link`	TEXT NOT NULL,
  `description`	TEXT NOT NULL,
  `image_uri`	TEXT,
  `source_id`	INTEGER NOT NULL UNIQUE,
  `favorite`  INTEGER NOT NULL DEFAULT 0,
  `archive`	  INTEGER NOT NULL DEFAULT 0,
  `always_dl`	INTEGER NOT NULL DEFAULT 0
);

INSERT INTO podcast (
  id,
  title,
  link,
  description,
  image_uri,
  source_id
) SELECT id,
  title,
  link,
  description,
  image_uri,
  source_id
FROM old_table;

Drop table old_table;