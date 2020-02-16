CREATE TABLE IF NOT EXISTS users
(
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  login TEXT NOT NULL,
  token TEXT NOT NULL,
  created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS tags
(
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  tag TEXT NOT NULL,
  user_id INTEGER REFERENCES users(id),
  used_at DATETIME,
  created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS links
(
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  url TEXT NOT NULL,
  description TEXT,
  user_id INTEGER REFERENCES users(id),
  hash TEXT NOT NULL,
  is_shared BOOLEAN NOT NULL DEFAULT FALSE,
  is_toread BOOLEAN NOT NULL DEFAULT FALSE,
  created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS links_tags
(
    link_id INTEGER REFERENCES links (id),
    tag_id  INTEGER REFERENCES tags (id)
);

-- this is to avoid duplicates of public tags, which are caused by
-- UNIQUE INDEX treating NULL values as non-identical ones

CREATE TRIGGER "uniqenull" BEFORE INSERT ON tags WHEN NEW.user_id IS NULL
BEGIN
    SELECT RAISE(IGNORE)
    WHERE EXISTS (SELECT 1 FROM tags c WHERE user_id IS NULL AND tag = NEW.tag);
END;

CREATE UNIQUE INDEX users_idx ON users(login);
CREATE UNIQUE INDEX links_idx ON links(url);
CREATE UNIQUE INDEX links_tags_idx ON links_tags(link_id, tag_id);
CREATE UNIQUE INDEX tags_user_idx ON tags(tag, user_id);
CREATE INDEX tags_used_idx ON tags(used_at);
