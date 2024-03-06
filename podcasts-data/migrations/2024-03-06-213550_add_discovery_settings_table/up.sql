CREATE TABLE discovery_settings (
       platform_id TEXT NOT NULL PRIMARY KEY,
       enabled BOOLEAN NOT NULL
);
INSERT INTO discovery_settings(platform_id, enabled) VALUES('fyyd.de', 0);
INSERT INTO discovery_settings(platform_id, enabled) VALUES('itunes.apple.com', 0);
