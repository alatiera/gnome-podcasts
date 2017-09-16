CREATE TABLE `episode` (
	`id`	INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT UNIQUE,
	`title`	TEXT NOT NULL,
	`uri`	TEXT NOT NULL,
	`local_uri`	TEXT,
	`description`	TEXT,
	`epoch`	INTEGER NOT NULL DEFAULT 0,
	`length`	INTEGER NOT NULL DEFAULT 0,
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
	`image_uri`	TEXT
);