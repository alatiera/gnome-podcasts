-- Till version 0.2 is released the plan is to edited directly and dont expect
-- any kind of non-braking changes.
-- After there is a stable prototype, Only diesel migrations will be used
-- in order to change the db schema.

CREATE TABLE `source` (
    `id`	INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT UNIQUE,
    `uri`	TEXT NOT NULL UNIQUE,
    `last_modified`	TEXT,
    `http_etag`	TEXT
);

CREATE TABLE `episode` (
    `id`	INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT UNIQUE,
    `title`	TEXT,
    `uri`	TEXT NOT NULL UNIQUE,
    `local_uri`	TEXT,
    `description`	TEXT,
    `published_date`	TEXT,
    `epoch`	INTEGER NOT NULL DEFAULT 0,
    `length`	INTEGER,
    `guid`	TEXT,
    `played`	INTEGER,
    `favorite`	INTEGER NOT NULL DEFAULT 0,
    `archive`	INTEGER NOT NULL DEFAULT 0,
    `podcast_id`	INTEGER NOT NULL
);

CREATE TABLE `podcast` (
    `id`	INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT UNIQUE,
    `title`	TEXT NOT NULL,
    `link`	TEXT NOT NULL,
    `description`	TEXT NOT NULL,
    `image_uri`	TEXT,
    `favorite`	INTEGER NOT NULL DEFAULT 0,
    `archive`	INTEGER NOT NULL DEFAULT 0,
    `always_dl`	INTEGER NOT NULL DEFAULT 0,
    `source_id`	INTEGER NOT NULL UNIQUE
);