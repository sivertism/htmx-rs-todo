PRAGMA foreign_keys = ON;
CREATE TABLE IF NOT EXISTS lists (
  id INTEGER PRIMARY KEY,
  name TEXT NOT NULL,
  created TEXT DEFAULT (strftime('%Y-%m-%d %H:%M:%S:%s', 'now', 'localtime') ),
  modified TEXT DEFAULT (strftime('%Y-%m-%d %H:%M:%S:%s', 'now', 'localtime') ) 
);

CREATE TABLE IF NOT EXISTS tasks (
  id INTEGER PRIMARY KEY,
  task TEXT NOT NULL,
  completed INTEGER NOT NULL DEFAULT 0 CHECK(completed IN (0,1)),
  position INTEGER NOT NULL DEFAULT 0,
  created TEXT DEFAULT (strftime('%Y-%m-%d %H:%M:%S:%s', 'now', 'localtime') ),
  modified TEXT DEFAULT (strftime('%Y-%m-%d %H:%M:%S:%s', 'now', 'localtime') ),
  list_id INTEGER, 
  FOREIGN KEY(list_id) REFERENCES lists(id)
  ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS grocy_credentials (
  id INTEGER PRIMARY KEY,
  url TEXT NOT NULL,
  api_key TEXT NOT NULL,
  created TEXT DEFAULT (strftime('%Y-%m-%d %H:%M:%S:%s', 'now', 'localtime') ),
  list_id INTEGER, 
  FOREIGN KEY(list_id) REFERENCES lists(id)
  ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS grocy_tasks_mapping (
  id INTEGER PRIMARY KEY,
  grocy_id INTEGER NOT NULL,
  created TEXT DEFAULT (strftime('%Y-%m-%d %H:%M:%S:%s', 'now', 'localtime') ),
  list_id INTEGER, 
  FOREIGN KEY(list_id) REFERENCES tasks(id)
  ON DELETE CASCADE
);

CREATE TRIGGER if not exists update_tasks_modified
BEFORE UPDATE
    ON tasks
BEGIN
    UPDATE tasks
       SET modified = strftime('%Y-%m-%d %H:%M:%S:%s', 'now', 'localtime') 
     WHERE id = old.id;
END;

CREATE TRIGGER if not exists update_lists_modified
BEFORE UPDATE
    ON tasks
BEGIN
    UPDATE lists
       SET modified = strftime('%Y-%m-%d %H:%M:%S:%s', 'now', 'localtime') 
     WHERE id = old.list_id;
END;
