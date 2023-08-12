CREATE TABLE settings_sync (
        server   TEXT NOT NULL,
        user     TEXT NOT NULL,
        active   BOOLEAN NOT NULL,
        last_sync INTEGER,
        PRIMARY KEY (server)
);

CREATE TABLE episodes_sync (
        ep_id INTEGER NOT NULL,
        action TEXT NOT NULL,
        timestamp INTEGER NOT NULL,
        start   INTEGER,
        position INTEGER,
        PRIMARY KEY (ep_id, action)
);

CREATE TABLE shows_sync (
        uri TEXT NOT NULL,
        new_uri TEXT,
        action TEXT NOT NULL,
        timestamp INTEGER NOT NULL,
        PRIMARY KEY (uri)
);
