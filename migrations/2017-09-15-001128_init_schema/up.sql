CREATE TABLE `source` (
	`id`	INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT UNIQUE,
	`uri`	TEXT NOT NULL UNIQUE,
	`last_modified`	TEXT,
	`http_etag`	TEXT
);

CREATE TABLE `episode` (
	`id`	INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT UNIQUE,
	`title`	TEXT NOT NULL,
	`uri`	TEXT NOT NULL,
	`local_uri`	TEXT,
	`description`	TEXT,
	`published_date` TEXT NOT NULL,
	`epoch`	INTEGER NOT NULL DEFAULT 0,
	`length`	INTEGER DEFAULT 0,
	`guid`	TEXT,
	`podcast_id`	INTEGER NOT NULL
);

CREATE TABLE `podcast` (
	`id`	INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT UNIQUE,
	`title`	TEXT NOT NULL,
	`uri`	TEXT UNIQUE NOT NULL,
	`link`	TEXT,
	`description`	TEXT,
	`image_uri`	TEXT,
	`source_id`	INTEGER NOT NULL
);