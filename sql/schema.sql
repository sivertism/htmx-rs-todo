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

-- Recipes table
CREATE TABLE IF NOT EXISTS recipes (
  id INTEGER PRIMARY KEY,
  title TEXT NOT NULL,
  instructions TEXT NOT NULL DEFAULT '',
  ingredients TEXT NOT NULL DEFAULT '',
  photo_url TEXT DEFAULT '',
  created TEXT DEFAULT (strftime('%Y-%m-%d %H:%M:%S:%s', 'now', 'localtime')),
  modified TEXT DEFAULT (strftime('%Y-%m-%d %H:%M:%S:%s', 'now', 'localtime'))
);

-- Meal plan entries
CREATE TABLE IF NOT EXISTS meal_plan (
  id INTEGER PRIMARY KEY,
  date TEXT NOT NULL, -- YYYY-MM-DD format
  meal_text TEXT NOT NULL, -- Either recipe title or free-form text
  recipe_id INTEGER, -- NULL for free-form entries
  created TEXT DEFAULT (strftime('%Y-%m-%d %H:%M:%S:%s', 'now', 'localtime')),
  FOREIGN KEY(recipe_id) REFERENCES recipes(id) ON DELETE SET NULL
);

-- Triggers for modification timestamps
CREATE TRIGGER IF NOT EXISTS update_recipes_modified
BEFORE UPDATE ON recipes
BEGIN
    UPDATE recipes SET modified = strftime('%Y-%m-%d %H:%M:%S:%s', 'now', 'localtime') 
    WHERE id = old.id;
END;

-- Recipe photos table for multiple images per recipe
CREATE TABLE IF NOT EXISTS recipe_photos (
  id INTEGER PRIMARY KEY,
  recipe_id INTEGER NOT NULL,
  filename TEXT NOT NULL,
  original_name TEXT NOT NULL,
  file_size INTEGER NOT NULL,
  mime_type TEXT NOT NULL,
  upload_order INTEGER NOT NULL DEFAULT 0,
  thumbnail_blob BLOB,
  created TEXT DEFAULT (strftime('%Y-%m-%d %H:%M:%S:%s', 'now', 'localtime')),
  FOREIGN KEY(recipe_id) REFERENCES recipes(id) ON DELETE CASCADE
);

-- Index for faster photo queries
CREATE INDEX IF NOT EXISTS idx_recipe_photos_recipe_id ON recipe_photos(recipe_id);
CREATE INDEX IF NOT EXISTS idx_recipe_photos_order ON recipe_photos(recipe_id, upload_order);
