create table if not exists lists (
  id INTEGER PRIMARY KEY,
  name TEXT NOT NULL,
  created TEXT DEFAULT (strftime('%Y-%m-%d %H:%M:%S:%s', 'now', 'localtime') ),
  modified TEXT DEFAULT (strftime('%Y-%m-%d %H:%M:%S:%s', 'now', 'localtime') ) 
);

create table if not exists tasks (
  id INTEGER PRIMARY KEY,
  task TEXT NOT NULL,
  completed INTEGER NOT NULL DEFAULT 0 CHECK(completed IN (0,1)),
  created TEXT DEFAULT (strftime('%Y-%m-%d %H:%M:%S:%s', 'now', 'localtime') ),
  modified TEXT DEFAULT (strftime('%Y-%m-%d %H:%M:%S:%s', 'now', 'localtime') ),
  list_id INTEGER, 
  FOREIGN KEY(list_id) REFERENCES tasks(id)
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
