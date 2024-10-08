use rusqlite;
use anyhow::Context;
use tokio_rusqlite::{params, Connection};
use crate::todo::Task;
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
                    debug!("{}", statement); })); 
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

    pub async fn get_grocy_credentials(&self, list_id: usize) -> Option<GrocyCredentials> {
        if let Ok(res) = self.connection
            .call(move |conn| {
                let res = conn.query_row(
                    "SELECT url, api_key FROM grocy_credentials WHERE list_id=(:list_id)",
                    &[(":list_id", &list_id)],
                    |row| {
                        Ok(GrocyCredentials {
                            url: row.get(0)?,
                            api_key: row.get(1)?,
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

    async fn create_task(
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
            .context("Failed to create task on db.")?;
        info!("Task item with id {} created", id);
        Ok(id)
    }


}


// Impl new (set up connection, run sql init code)

// pub fn init_db() -> Connection {
//     // Create, or connect to a local SQLite database to store the tasks
//     let conn = Connection::open("todos.db").expect("Failed to open database");
// 
//     let sql_schema = include_str!("../sql/schema.sql");
//     conn.execute_batch(sql_schema).expect("Failed to execute schema");
// 
//     conn
// }
// 
// pub fn delete_task (id: usize) {
//     let conn = Connection::open("todos.db").expect("Failed to open database");
//     match conn.execute("DELETE FROM tasks WHERE id=(?1)", &[&id]) {
//         Ok(updated) => {
//             println!("{} rows were deleted", updated);
//         }
//         Err(err) => {
//             println!("Delete failed: {}", err);
//         } ,
//     }
// }
// 
// // returns toggled task
// pub fn toggle_task (id: usize) -> Result<Task> {
//     let conn = Connection::open("todos.db").expect("Failed to open database");
//     match conn.execute("UPDATE tasks SET completed = ((completed | 1) - (completed & 1)) WHERE id=(?1)", &[&id]) {
//         Ok(updated) => {
//             println!("{} rows were updated", updated);
//         }
//         Err(err) => {
//             println!("Update failed: {}", err);
//         }
//     }
// 
//     let task = conn.query_row("SELECT * FROM tasks WHERE id=(?1)", &[&id], | row | Ok(Task{id: row.get(0)?, text: row.get(1)?, completed: row.get(2)?}))?;
//     Ok(task)
// }
// 
// // Returns the id of the newly created task
// pub fn create_task (text: String) -> usize {
//     let conn = Connection::open("todos.db").expect("Failed to open database");
//     match conn.execute("INSERT INTO tasks (task) values (?1)", &[&text]) {
//         Ok(updated) => {
//             println!("{} rows were inserted", updated);
//         }
//         Err(err) => {
//             println!("Update failed: {}", err);
//         } ,
//     }
//     conn.last_insert_rowid() as usize
// }
// 
// pub fn get_tasks (completed: bool) -> Result<Vec<Task>>{ 
//     let conn = Connection::open("todos.db").expect("Failed to open database");
//     let mut stmt = conn.prepare(
//         "SELECT * FROM tasks WHERE completed=?1 ORDER BY modified DESC;",
//         )?;
// 
//     let rows = stmt.query_map(&[&completed], |row| Ok(Task { id: row.get(0)?, text: row.get(1)?, completed: row.get(2)?}))?;
// 
//     let mut tasks = Vec::new();
//     for r in rows {
//         tasks.push(r?);
//     }
//     Ok(tasks)
// }
