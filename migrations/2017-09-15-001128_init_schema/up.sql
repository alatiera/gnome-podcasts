CREATE TABLE `Episode` (
	`id`	INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT UNIQUE,
	`title`	TEXT NOT NULL,
	`desrciption`	TEXT,
	`uri`	TEXT NOT NULL,
	`local_uri`	TEXT,
	`thumbnail`	TEXT,
	`lenght`	INTEGER,
	`guid`	TEXT,
	`podcast_id`	INTEGER NOT NULL
);

CREATE TABLE `Podcast` (
	`id`	INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT UNIQUE,
	`title`	TEXT NOT NULL,
	`uri`	TEXT NOT NULL,
	`link`	TEXT,
	`description`	TEXT,
	`last_modified`	TEXT,
	`http_etag`	INTEGER,
	`image_uri`	TEXT,
	`image_local`	TEXT
);