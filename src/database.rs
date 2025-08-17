use rusqlite;
use anyhow::Context;
use tokio_rusqlite::Connection;
use crate::todo::{Task, List, Recipe, MealPlanEntry, RecipePhoto};
use tracing::{info, warn};
use crate::grocy::*;

#[derive(Clone)]
pub struct Database {
    connection : Connection,
}

impl Database {

    pub async fn new(dbfile: std::path::PathBuf) -> anyhow::Result<Database> {
        let dbconn = Connection::open(dbfile)
            .await
            .context("Open database")?;

        // Insert basic tracing function to print sql queries to console
        dbconn
            .call(|conn| {
                conn.trace(Some(|statement| {
                    info!("{}", statement); })); 
                    Ok(()) 
            })
            .await
            .context("Add tracing function")?;

        // Initialize database
        dbconn
            .call(|conn| {
                let sql_schema = include_str!("../sql/schema.sql");
                conn.execute_batch(sql_schema)
                    .expect("Failed to execute database schema");
                Ok(())
            })
            .await?;
        Ok(Database { connection: dbconn })
    }

    pub async fn delete_task(&self, id: usize) -> anyhow::Result<()> {
        self.connection
            .call(
                move |conn| {
                    match conn.execute("DELETE FROM tasks WHERE id=(?1)", &[&id]) {
                        Ok(_) => Ok(()),
                        Err(err) => {
                            warn!("Delete task failed: {}", err);
                            Err(err.into())
                        }
                    }
                },
            )
            .await
            .context("Delete task")
    }

    pub async fn delete_list(&self, id: usize) -> anyhow::Result<()> {
        self.connection
            .call(
                move |conn| {
                    match conn.execute("DELETE FROM lists WHERE id=(?1)", &[&id]) {
                        Ok(_) => Ok(()),
                        Err(err) => {
                            warn!("Delete list failed: {}", err);
                            Err(err.into())
                        }
                    }
                },
            )
            .await
            .context("Delete list")
    }

    pub async fn get_task(&self, id: usize) -> anyhow::Result<Task> {
        Ok(self.connection
            .call(move |conn| {
                let t = conn.query_row("SELECT id, task, completed, list_id, position FROM tasks WHERE id=(?1)", &[&id], 
                |row| {
                    Ok(Task {
                        id: row.get(0).expect("Failed to get id, corrupt database?"),
                        text: row.get(1).expect("Failed to get task, corrupt database?"),
                        completed: row.get(2).expect("Failed to get completed, corrupt database?"),
                        list_id: row.get(3).expect("Failed to get list_id, corrupt database?"),
                        position: row.get(4).ok(),
                    })
                });
                Ok(t)
                }
            ).await.context("Get task")??)
    }

    pub async fn get_tasks(&self, list_id: usize) -> anyhow::Result<Vec<Task>> {
        Ok(self.connection
            .call(move |conn| {
                let mut stmt = conn.prepare(
                    "SELECT tasks.id, tasks.task, tasks.completed, tasks.list_id, tasks.position 
                    FROM tasks 
                    INNER JOIN lists ON lists.id=tasks.list_id 
                    WHERE lists.id=(:list_id) 
                    ORDER BY tasks.position ASC NULLS LAST, tasks.completed ASC, tasks.modified DESC;",
                )?;
                let rows = stmt.query_map(&[(":list_id", &list_id)], |row| {
                    Ok(Task {
                        id: row.get(0).expect("Failed to get id, corrupt database?"),
                        text: row.get(1).expect("Failed to get task, corrupt database?"),
                        completed: row.get(2).expect("Failed to get completed, corrupt database?"),
                        list_id: row.get(3).expect("Failed to get list_id, corrupt database?"),
                        position: row.get(4).ok(),
                    })
                })?;
                let mut tasks = Vec::new();
                for r in rows {
                    tasks.push(r?);
                }
                Ok(tasks)
            })
            .await
            .context("Failed to get tasks")?)
    }

    pub async fn get_grocy_credentials(&self, list_id: usize) -> Option<GrocyCredentials> {
        if let Ok(res) = self.connection
            .call(move |conn| {
                let res = conn.query_row(
                    "SELECT url, api_key FROM grocy_credentials WHERE list_id=(:list_id)",
                    &[(":list_id", &list_id)],
                    |row| {
                        Ok(GrocyCredentials {
                            url: row.get(0).expect("Failed to get row value, corrupt database?"),
                            api_key: row.get(1).expect("Failed to get row value, corrupt database?"),
                        })
                    },
                )?;
                Ok(res)
            })
            .await 
        {
            return Some(res);
        }
        return None;
    }

    pub async fn create_task(
        &self,
        text: String,
        list_id: usize,
    ) -> anyhow::Result<usize> {
        info!("Inserting task item with list_id {}", list_id);

        let id = self
            .connection
            .call(move |conn| {
                // Get the next position for this list
                let next_position: i32 = conn.query_row(
                    "SELECT COALESCE(MAX(position), -1) + 1 FROM tasks WHERE list_id = ?1",
                    &[&list_id],
                    |row| row.get(0)
                ).unwrap_or(0);

                match conn.execute(
                    "INSERT INTO tasks (task, list_id, position) values (?1, ?2, ?3)",
                    rusqlite::params![&text.clone(), &list_id, &next_position],
                ) {
                    Ok(updated) => {
                        info!("{} rows were inserted", updated);
                        Ok(conn.last_insert_rowid() as usize)
                    }
                    Err(err) => {
                        warn!("Create task failed: {}", err);
                        Err(err.into())
                    }
                }
            })
            .await
            .context("Create task on db.")?;
        info!("Task item with id {} created", id);
        Ok(id)
    }

    pub async fn toggle_task_completed(&self, task_id: usize) -> anyhow::Result<()>{
        
        self
            .connection
            .call(move |conn| {
                match conn.execute(
                    "UPDATE tasks 
                               SET completed = ((completed | 1) - (completed & 1)) 
                               WHERE id=(?1)",
                    &[&task_id],
                ) {
                    Ok(updated) => {
                        info!("{} rows were updated", updated);
                        Ok(())
                    }
                    Err(err) => {
                        warn!("Failed to update task: {}", err);
                        Err(err.into())
                    }
                }
            })
            .await
            .context("Toggle task complete")
    }

    pub async fn create_list(
        &self,
        name: String,
        grocy_credentials: Option<&GrocyCredentials>,
        ) -> anyhow::Result<usize> {
        let id = self
            .connection
            .call(move |conn| {
                // Create the list
                match conn.execute("INSERT INTO lists (name) values (?1)", &[&name]) {
                    Ok(_) => {},
                    Err(err) => {
                        warn!("Create list failed: {}", err);
                        return Err(err.into());
                    }
                }
                Ok(conn.last_insert_rowid() as usize)
            })
            .await
            .context("Create list on db.")?;
        if let Some(gc) = grocy_credentials {
            if gc.url != "" && gc.api_key != "" {
            let gc = gc.clone();
            info!("Inserting Grocy credentials for {}", gc.url);
            self
                .connection
                .call(move |conn| {
                    // Create the grocy credentials
                    match conn.execute(
                        "INSERT INTO grocy_credentials (url, api_key, list_id) values (?1, ?2, ?3)",
                        rusqlite::params![&gc.url, &gc.api_key, &id],
                    ) {
                        Ok(_) => {
                            info!("Grocy credentials stored for list {}", id);
                        }
                        Err(err) => {
                            warn!("Failed to store Grocy credentials: {}", err);
                            return Err(err.into());
                        }
                    }
                    Ok(())
                })
                .await
                .context("Store Grocy credentials.")?;
            }
        }
        Ok(id)
    }

    pub async fn get_list(&self, id: usize) -> anyhow::Result<List> 
    {
        let list = self
            .connection
            .call(move |conn| {
                Ok(conn.query_row(
                    "SELECT * FROM lists WHERE id=(?1)",
                    &[&id],
                    |row| {
                        Ok(List {
                            id: row.get(0).expect("Failed to get row value, corrupt database?"),
                            name: row.get(1).expect("Failed to get row value, corrupt database?"),
                        })
                    },
                ))
            })
            .await
            .context("Retrieve list")?;
        Ok(list?)
    }

    pub async fn get_lists(&self) -> anyhow::Result<Vec<List>> {
        Ok(self.connection
            .call(move |conn| {
                let mut stmt = conn.prepare(
                    "SELECT lists.id, lists.name FROM lists;",
                )?;
                let rows = stmt.query_map([], |row| {
                    Ok(List {
                        id: row.get(0).expect("Failed to get row value, corrupt database?"),
                        name: row.get(1).expect("Failed to get row value, corrupt database?"),
                    })
                })?;
                let mut lists = Vec::new();
                for r in rows {
                    lists.push(r?);
                }
                Ok(lists)
            })
            .await
            .context("Failed to get lists")?)
    }

    pub async fn reorder(&self, list_id: usize, order: Vec<u64>) -> anyhow::Result<()> {
        if order.is_empty() {
            return Ok(());
        }

        self.connection
            .call(move |conn| {
                let tx = conn.transaction()?;
                
                // Update positions for the reordered tasks
                for (position, task_id) in order.iter().enumerate() {
                    tx.execute(
                        "UPDATE tasks SET position = ?1 WHERE id = ?2 AND list_id = ?3",
                        rusqlite::params![position as i32, task_id, list_id],
                    )?;
                }
                
                // Fix positions for any tasks not in the order (put them at the end)
                let order_placeholders = order.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
                let mut params: Vec<rusqlite::types::Value> = order.iter().map(|&id| (id as i64).into()).collect();
                params.push((list_id as i64).into());
                params.push((order.len() as i32).into());
                
                let query = format!(
                    "UPDATE tasks SET position = position + ? WHERE list_id = ? AND id NOT IN ({})",
                    order_placeholders
                );
                
                // Shift existing tasks that weren't reordered to the end
                tx.execute(&query, rusqlite::params_from_iter(params))?;
                
                tx.commit()?;
                Ok(())
            })
            .await
            .context("Reorder tasks")
    }

    // Recipe operations
    pub async fn create_recipe(
        &self,
        title: String,
        instructions: String,
        ingredients: String,
    ) -> anyhow::Result<usize> {
        info!("Creating recipe: {}", title);

        let id = self
            .connection
            .call(move |conn| {
                match conn.execute(
                    "INSERT INTO recipes (title, instructions, ingredients) VALUES (?1, ?2, ?3)",
                    rusqlite::params![&title, &instructions, &ingredients],
                ) {
                    Ok(_) => Ok(conn.last_insert_rowid() as usize),
                    Err(err) => {
                        warn!("Create recipe failed: {}", err);
                        Err(err.into())
                    }
                }
            })
            .await
            .context("Create recipe")?;
        
        info!("Recipe created with id {}", id);
        Ok(id)
    }

    pub async fn get_recipe(&self, id: usize) -> anyhow::Result<Recipe> {
        self.connection
            .call(move |conn| {
                let recipe = conn.query_row(
                    "SELECT id, title, instructions, ingredients FROM recipes WHERE id = ?1",
                    &[&id],
                    |row| {
                        Ok(Recipe {
                            id: row.get(0)?,
                            title: row.get(1)?,
                            instructions: row.get(2)?,
                            ingredients: row.get(3)?,
                        })
                    },
                )?;
                Ok(recipe)
            })
            .await
            .context("Get recipe")
    }

    pub async fn get_recipes(&self) -> anyhow::Result<Vec<Recipe>> {
        self.connection
            .call(move |conn| {
                let mut stmt = conn.prepare(
                    "SELECT id, title, instructions, ingredients FROM recipes ORDER BY modified DESC"
                )?;
                let rows = stmt.query_map([], |row| {
                    Ok(Recipe {
                        id: row.get(0)?,
                        title: row.get(1)?,
                        instructions: row.get(2)?,
                        ingredients: row.get(3)?,
                    })
                })?;
                let mut recipes = Vec::new();
                for r in rows {
                    recipes.push(r?);
                }
                Ok(recipes)
            })
            .await
            .context("Get recipes")
    }

    pub async fn update_recipe(
        &self,
        id: usize,
        title: String,
        instructions: String,
        ingredients: String,
    ) -> anyhow::Result<()> {
        self.connection
            .call(move |conn| {
                match conn.execute(
                    "UPDATE recipes SET title = ?1, instructions = ?2, ingredients = ?3 WHERE id = ?4",
                    rusqlite::params![&title, &instructions, &ingredients, &id],
                ) {
                    Ok(_) => Ok(()),
                    Err(err) => {
                        warn!("Update recipe failed: {}", err);
                        Err(err.into())
                    }
                }
            })
            .await
            .context("Update recipe")
    }

    pub async fn delete_recipe(&self, id: usize) -> anyhow::Result<()> {
        self.connection
            .call(move |conn| {
                match conn.execute("DELETE FROM recipes WHERE id = ?1", &[&id]) {
                    Ok(_) => Ok(()),
                    Err(err) => {
                        warn!("Delete recipe failed: {}", err);
                        Err(err.into())
                    }
                }
            })
            .await
            .context("Delete recipe")
    }

    // Meal plan operations
    pub async fn create_meal_plan_entry(
        &self,
        date: String,
        meal_text: String,
        recipe_id: Option<usize>,
    ) -> anyhow::Result<usize> {
        info!("Creating meal plan entry for {}: {}", date, meal_text);

        let id = self
            .connection
            .call(move |conn| {
                match conn.execute(
                    "INSERT INTO meal_plan (date, meal_text, recipe_id) VALUES (?1, ?2, ?3)",
                    rusqlite::params![&date, &meal_text, &recipe_id],
                ) {
                    Ok(_) => Ok(conn.last_insert_rowid() as usize),
                    Err(err) => {
                        warn!("Create meal plan entry failed: {}", err);
                        Err(err.into())
                    }
                }
            })
            .await
            .context("Create meal plan entry")?;
        
        info!("Meal plan entry created with id {}", id);
        Ok(id)
    }

    pub async fn get_meal_plan_for_week(&self, start_date: String) -> anyhow::Result<Vec<MealPlanEntry>> {
        self.connection
            .call(move |conn| {
                let mut stmt = conn.prepare(
                    "SELECT id, date, meal_text, recipe_id FROM meal_plan 
                     WHERE date >= ?1 AND date < date(?1, '+7 days') 
                     ORDER BY date ASC"
                )?;
                let rows = stmt.query_map(&[&start_date], |row| {
                    Ok(MealPlanEntry {
                        id: row.get(0)?,
                        date: row.get(1)?,
                        meal_text: row.get(2)?,
                        recipe_id: row.get(3)?,
                    })
                })?;
                let mut entries = Vec::new();
                for r in rows {
                    entries.push(r?);
                }
                Ok(entries)
            })
            .await
            .context("Get meal plan for week")
    }

    pub async fn delete_meal_plan_entry(&self, id: usize) -> anyhow::Result<()> {
        self.connection
            .call(move |conn| {
                match conn.execute("DELETE FROM meal_plan WHERE id = ?1", &[&id]) {
                    Ok(_) => Ok(()),
                    Err(err) => {
                        warn!("Delete meal plan entry failed: {}", err);
                        Err(err.into())
                    }
                }
            })
            .await
            .context("Delete meal plan entry")
    }

    pub async fn update_meal_plan_entry(
        &self,
        id: usize,
        meal_text: String,
        recipe_id: Option<usize>,
    ) -> anyhow::Result<()> {
        self.connection
            .call(move |conn| {
                match conn.execute(
                    "UPDATE meal_plan SET meal_text = ?1, recipe_id = ?2 WHERE id = ?3",
                    rusqlite::params![&meal_text, &recipe_id, &id],
                ) {
                    Ok(_) => Ok(()),
                    Err(err) => {
                        warn!("Update meal plan entry failed: {}", err);
                        Err(err.into())
                    }
                }
            })
            .await
            .context("Update meal plan entry")
    }

    pub async fn get_meal_plan_entry(&self, id: usize) -> anyhow::Result<MealPlanEntry> {
        self.connection
            .call(move |conn| {
                let entry = conn.query_row(
                    "SELECT id, date, meal_text, recipe_id FROM meal_plan WHERE id = ?1",
                    &[&id],
                    |row| {
                        Ok(MealPlanEntry {
                            id: row.get(0)?,
                            date: row.get(1)?,
                            meal_text: row.get(2)?,
                            recipe_id: row.get(3)?,
                        })
                    },
                )?;
                Ok(entry)
            })
            .await
            .context("Get meal plan entry")
    }

    // Recipe photo operations
    pub async fn create_recipe_photo(
        &self,
        recipe_id: usize,
        filename: String,
        original_name: String,
        file_size: i64,
        mime_type: String,
        upload_order: i32,
        thumbnail_blob: Option<Vec<u8>>,
    ) -> anyhow::Result<usize> {
        info!("Creating recipe photo: {} for recipe {}", original_name, recipe_id);

        let id = self
            .connection
            .call(move |conn| {
                match conn.execute(
                    "INSERT INTO recipe_photos (recipe_id, filename, original_name, file_size, mime_type, upload_order, thumbnail_blob) 
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                    rusqlite::params![&recipe_id, &filename, &original_name, &file_size, &mime_type, &upload_order, &thumbnail_blob],
                ) {
                    Ok(_) => Ok(conn.last_insert_rowid() as usize),
                    Err(err) => {
                        warn!("Create recipe photo failed: {}", err);
                        Err(err.into())
                    }
                }
            })
            .await
            .context("Create recipe photo")?;
        
        info!("Recipe photo created with id {}", id);
        Ok(id)
    }

    pub async fn get_recipe_photos(&self, recipe_id: usize) -> anyhow::Result<Vec<RecipePhoto>> {
        self.connection
            .call(move |conn| {
                let mut stmt = conn.prepare(
                    "SELECT id, recipe_id, filename, original_name, file_size, mime_type, upload_order, thumbnail_blob 
                     FROM recipe_photos WHERE recipe_id = ?1 ORDER BY upload_order ASC"
                )?;
                let rows = stmt.query_map(&[&recipe_id], |row| {
                    Ok(RecipePhoto {
                        id: row.get(0)?,
                        recipe_id: row.get(1)?,
                        filename: row.get(2)?,
                        original_name: row.get(3)?,
                        file_size: row.get(4)?,
                        mime_type: row.get(5)?,
                        upload_order: row.get(6)?,
                        thumbnail_blob: row.get(7)?,
                    })
                })?;
                let mut photos = Vec::new();
                for r in rows {
                    photos.push(r?);
                }
                Ok(photos)
            })
            .await
            .context("Get recipe photos")
    }

    pub async fn get_recipe_first_photo(&self, recipe_id: usize) -> anyhow::Result<Option<RecipePhoto>> {
        self.connection
            .call(move |conn| {
                let result = conn.query_row(
                    "SELECT id, recipe_id, filename, original_name, file_size, mime_type, upload_order, thumbnail_blob 
                     FROM recipe_photos WHERE recipe_id = ?1 ORDER BY upload_order ASC LIMIT 1",
                    &[&recipe_id],
                    |row| {
                        Ok(RecipePhoto {
                            id: row.get(0)?,
                            recipe_id: row.get(1)?,
                            filename: row.get(2)?,
                            original_name: row.get(3)?,
                            file_size: row.get(4)?,
                            mime_type: row.get(5)?,
                            upload_order: row.get(6)?,
                            thumbnail_blob: row.get(7)?,
                        })
                    },
                );
                match result {
                    Ok(photo) => Ok(Some(photo)),
                    Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                    Err(err) => Err(err.into()),
                }
            })
            .await
            .context("Get recipe first photo")
    }

    pub async fn delete_recipe_photo(&self, id: usize) -> anyhow::Result<()> {
        self.connection
            .call(move |conn| {
                match conn.execute("DELETE FROM recipe_photos WHERE id = ?1", &[&id]) {
                    Ok(_) => Ok(()),
                    Err(err) => {
                        warn!("Delete recipe photo failed: {}", err);
                        Err(err.into())
                    }
                }
            })
            .await
            .context("Delete recipe photo")
    }

    pub async fn delete_recipe_photos_by_recipe(&self, recipe_id: usize) -> anyhow::Result<()> {
        self.connection
            .call(move |conn| {
                match conn.execute("DELETE FROM recipe_photos WHERE recipe_id = ?1", &[&recipe_id]) {
                    Ok(_) => Ok(()),
                    Err(err) => {
                        warn!("Delete recipe photos failed: {}", err);
                        Err(err.into())
                    }
                }
            })
            .await
            .context("Delete recipe photos by recipe")
    }

    pub async fn get_next_photo_order(&self, recipe_id: usize) -> anyhow::Result<i32> {
        self.connection
            .call(move |conn| {
                let order: i32 = conn.query_row(
                    "SELECT COALESCE(MAX(upload_order), -1) + 1 FROM recipe_photos WHERE recipe_id = ?1",
                    &[&recipe_id],
                    |row| row.get(0)
                ).unwrap_or(0);
                Ok(order)
            })
            .await
            .context("Get next photo order")
    }

    pub async fn get_recipe_photo_by_id(&self, photo_id: usize) -> anyhow::Result<Option<RecipePhoto>> {
        self.connection
            .call(move |conn| {
                let result = conn.query_row(
                    "SELECT id, recipe_id, filename, original_name, file_size, mime_type, upload_order, thumbnail_blob 
                     FROM recipe_photos WHERE id = ?1",
                    &[&photo_id],
                    |row| {
                        Ok(RecipePhoto {
                            id: row.get(0)?,
                            recipe_id: row.get(1)?,
                            filename: row.get(2)?,
                            original_name: row.get(3)?,
                            file_size: row.get(4)?,
                            mime_type: row.get(5)?,
                            upload_order: row.get(6)?,
                            thumbnail_blob: row.get(7)?,
                        })
                    },
                );
                match result {
                    Ok(photo) => Ok(Some(photo)),
                    Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                    Err(err) => Err(err.into()),
                }
            })
            .await
            .context("Get recipe photo by id")
    }
}
