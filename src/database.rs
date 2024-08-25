//use rusqlite::NO_PARAMS;
use rusqlite::{Connection, Result};
use crate::todo::Task;

struct Database {
    connection : Connection,
}


// Impl new (set up connection, run sql init code)

pub fn init_db() -> Connection {
    // Create, or connect to a local SQLite database to store the tasks
    let conn = Connection::open("todos.db").expect("Failed to open database");

    let sql_schema = include_str!("../sql/schema.sql");
    conn.execute_batch(sql_schema).expect("Failed to execute schema");

    conn
}

pub fn delete_task (id: usize) {
    let conn = Connection::open("todos.db").expect("Failed to open database");
    match conn.execute("DELETE FROM tasks WHERE id=(?1)", &[&id]) {
        Ok(updated) => {
            println!("{} rows were deleted", updated);
        }
        Err(err) => {
            println!("Delete failed: {}", err);
        } ,
    }
}

// returns toggled task
pub fn toggle_task (id: usize) -> Result<Task> {
    let conn = Connection::open("todos.db").expect("Failed to open database");
    match conn.execute("UPDATE tasks SET completed = ((completed | 1) - (completed & 1)) WHERE id=(?1)", &[&id]) {
        Ok(updated) => {
            println!("{} rows were updated", updated);
        }
        Err(err) => {
            println!("Update failed: {}", err);
        }
    }

    let task = conn.query_row("SELECT * FROM tasks WHERE id=(?1)", &[&id], | row | Ok(Task{id: row.get(0)?, text: row.get(1)?, completed: row.get(2)?}))?;
    Ok(task)
}

// Returns the id of the newly created task
pub fn create_task (text: String) -> usize {
    let conn = Connection::open("todos.db").expect("Failed to open database");
    match conn.execute("INSERT INTO tasks (task) values (?1)", &[&text]) {
        Ok(updated) => {
            println!("{} rows were inserted", updated);
        }
        Err(err) => {
            println!("Update failed: {}", err);
        } ,
    }
    conn.last_insert_rowid() as usize
}

pub fn get_tasks (completed: bool) -> Result<Vec<Task>>{ 
    let conn = Connection::open("todos.db").expect("Failed to open database");
    let mut stmt = conn.prepare(
        "SELECT * FROM tasks WHERE completed=?1 ORDER BY modified DESC;",
        )?;

    let rows = stmt.query_map(&[&completed], |row| Ok(Task { id: row.get(0)?, text: row.get(1)?, completed: row.get(2)?}))?;

    let mut tasks = Vec::new();
    for r in rows {
        tasks.push(r?);
    }
    Ok(tasks)
}
