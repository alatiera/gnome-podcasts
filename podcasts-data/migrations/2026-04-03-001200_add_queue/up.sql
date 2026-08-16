CREATE TABLE queue (
    id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    episode_id INTEGER NOT NULL REFERENCES episodes(id) ON DELETE CASCADE,
    position REAL NOT NULL
);

CREATE UNIQUE INDEX queue_position_unique ON queue(position);
CREATE UNIQUE INDEX queue_episode_unique ON queue(episode_id);
