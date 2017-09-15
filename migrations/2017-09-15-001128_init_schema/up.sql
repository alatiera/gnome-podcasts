CREATE TABLE `episode` (
	`id`	INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT UNIQUE,
	`title`	TEXT NOT NULL,
	`uri`	TEXT NOT NULL,
	`description`	TEXT,
	`local_uri`	TEXT,
	`thumbnail`	TEXT,
	`lenght`	INTEGER,
	`guid`	TEXT,
	`podcast_id`	INTEGER NOT NULL
);

CREATE TABLE `podcast` (
	`id`	INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT UNIQUE,
	`title`	TEXT NOT NULL,
	`uri`	TEXT NOT NULL,
	`link`	TEXT,
	`description`	TEXT,
	`last_modified`	TEXT,
	`http_etag`	TEXT,
	`image_uri`	TEXT,
	`image_local`	TEXT
);