CREATE TABLE `source` (
	`id`	INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT UNIQUE,
	`uri`	TEXT NOT NULL UNIQUE,
	`last_modified`	TEXT,
	`http_etag`	TEXT
);

CREATE TABLE `episode` (
	`id`	INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT UNIQUE,
	`title`	TEXT,
	`uri`	TEXT UNIQUE,
	`local_uri`	TEXT,
	`description`	TEXT,
	`published_date` TEXT ,
	`epoch`	INTEGER NOT NULL DEFAULT 0,
	`length`	INTEGER,
	`guid`	TEXT,
	`podcast_id`	INTEGER NOT NULL
);

CREATE TABLE `podcast` (
	`id`	INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT UNIQUE,
	`title`	TEXT NOT NULL UNIQUE,
	`link`	TEXT NOT NULL,
	`description`	TEXT NOT NULL,
	`image_uri`	TEXT,
	`source_id`	INTEGER NOT NULL
);