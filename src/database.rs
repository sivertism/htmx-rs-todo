use rusqlite;
use anyhow::Context;
use tokio_rusqlite::Connection;
use crate::todo::{Task, List};
use tracing::{info, warn, debug};
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
        Ok(self.connection
            .call(
                move |conn| match conn.execute("DELETE FROM tasks WHERE id=(?1)", &[&id]) {
                    Ok(_n_updated) => {
                        Ok(())
                    }
                    Err(err) => {
                        println!("Delete failed: {}", err);
                        Ok(())
                    }
                },
            )
            .await
            .context("Delete task")?)
    }

    pub async fn get_task(&self, id: usize) -> anyhow::Result<Task> {
        Ok(self.connection
            .call(move |conn| {
                let t = conn.query_row("SELECT * FROM tasks WHERE id=(?1)", &[&id], 
                |row| {
                    Ok(Task {
                        id: row.get(0).expect("Failed to get id, corrupt database?"),
                        text: row.get(1).expect("Failed to get id, corrupt database?"),
                        completed: row.get(2).expect("Failed to get id, corrupt database?"),
                        list_id: id as usize,
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
                    "SELECT tasks.id, tasks.task, tasks.completed 
                    FROM tasks 
                    INNER JOIN lists ON lists.id=tasks.list_id 
                    WHERE lists.id=(:list_id) 
                    ORDER BY tasks.completed, tasks.modified DESC;",
                )?;
                let rows = stmt.query_map(&[(":list_id", &list_id)], |row| {
                    Ok(Task {
                        id: row.get(0).expect("Failed to get id, corrupt database?"),
                        text: row.get(1).expect("Failed to get id, corrupt database?"),
                        completed: row.get(2).expect("Failed to get id, corrupt database?"),
                        list_id: list_id,
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
                match conn.execute(
                    "INSERT INTO tasks (task, list_id) values (?1, ?2)",
                    rusqlite::params![&text.clone(), &list_id],
                ) {
                    Ok(updated) => {
                        info!("{} rows were inserted", updated);
                    }
                    Err(err) => {
                        panic!("Create task failed: {}", err);
                    }
                }
                Ok(conn.last_insert_rowid() as usize)
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
                    }
                    Err(err) => {
                        panic!("Failed to update row with {}", err);
                    }
                };
                Ok(())
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
                    Ok(_n_updated) => {}
                    Err(err) => {
                        warn!("Create list failed: {}", err);
                    }
                }
                Ok(conn.last_insert_rowid() as usize)
            })
            .await
            .context("Create list on db.")?;
        if let Some(gc) = grocy_credentials {
            let gc = gc.clone();
            info!("Inserting Grocy credentials for {}", gc.url);
            self
                .connection
                .call(move |conn| {
                    // Create the list
                    match conn.execute(
                        "INSERT INTO grocy_credentials (url, api_key, list_id) values (?1, ?2, ?3)",
                        rusqlite::params![&gc.url, &gc.api_key, &id],
                    ) {
                        Ok(updated) => {
                            info!("{} rows were inserted", updated);
                        }
                        Err(err) => {
                            panic!("Failed to store Grocy credentials: {}", err);
                        }
                    }
                    Ok(())
                })
                .await
                .context("Store Grocy credentials.")?;
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
}
