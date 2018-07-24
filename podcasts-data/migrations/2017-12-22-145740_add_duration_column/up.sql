ALTER TABLE episode RENAME TO old_table;

CREATE TABLE episode (
	title	TEXT NOT NULL,
	uri	TEXT,
	local_uri	TEXT,
	description	TEXT,
	published_date	TEXT,
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

INSERT INTO episode (title, uri, local_uri, description, published_date, epoch, length, guid, played, favorite, archive, podcast_id)
SELECT title, uri, local_uri, description, published_date, epoch, length, guid, played, favorite, archive, podcast_id
FROM old_table;
Drop table old_table;