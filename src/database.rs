//use rusqlite::NO_PARAMS;
use rusqlite::{Connection, Result};
use crate::todo::Todo;

fn get_conn() -> Connection {
    // Create, or connect to a local SQLite database to store the todos
    let conn = Connection::open("todos.db").expect("Failed to open database");

    conn.execute(
        "create table if not exists todos (
          id INTEGER PRIMARY KEY,
          task TEXT NOT NULL,
          completed INTEGER NOT NULL DEFAULT 0 CHECK(completed IN (0,1)),
          created TEXT DEFAULT (strftime('%Y-%m-%d %H:%M:%S:%s', 'now', 'localtime') ),
          modified TEXT DEFAULT (strftime('%Y-%m-%d %H:%M:%S:%s', 'now', 'localtime') ) 
        )
        ",
        [],
    ).expect("Failed to open database");

    conn.execute(
        "
        CREATE TRIGGER if not exists update_todos_modified
        BEFORE UPDATE
            ON todos
        BEGIN
            UPDATE todos
               SET modified = strftime('%Y-%m-%d %H:%M:%S:%s', 'now', 'localtime') 
             WHERE id = old.id;
        END;
        ",
        [],
    ).expect("Failed to create triggers on database");

    conn
}

pub fn delete_todo (id: usize) {
    let conn = get_conn();
    match conn.execute("DELETE FROM todos WHERE id=(?1)", &[&id]) {
        Ok(updated) => {
            println!("{} rows were deleted", updated);
        }
        Err(err) => {
            println!("Delete failed: {}", err);
        } ,
    }
}

// returns toggled todo
pub fn toggle_todo (id: usize) -> Result<Todo> {
    let conn = get_conn();
    match conn.execute("UPDATE todos SET completed = ((completed | 1) - (completed & 1)) WHERE id=(?1)", &[&id]) {
        Ok(updated) => {
            println!("{} rows were updated", updated);
        }
        Err(err) => {
            println!("Update failed: {}", err);
        } ,
    }

    let todo = conn.query_row("SELECT * FROM todos WHERE id=(?1)", &[&id], | row | Ok(Todo{id: row.get(0)?, text: row.get(1)?, completed: row.get(2)?}))?;
    Ok(todo)
}

// Returns the id of the newly created todo
pub fn create_todo (text: String) -> usize {
    let conn = get_conn();
    match conn.execute("INSERT INTO todos (task) values (?1)", &[&text]) {
        Ok(updated) => {
            println!("{} rows were inserted", updated);
        }
        Err(err) => {
            println!("Update failed: {}", err);
        } ,
    }
    conn.last_insert_rowid() as usize
}

pub fn get_todos (completed: bool) -> Result<Vec<Todo>>{ 
    let conn = get_conn();
    let mut stmt = conn.prepare(
        "SELECT * FROM todos WHERE completed=?1 ORDER BY modified DESC;",
        )?;

    let rows = stmt.query_map(&[&completed], |row| Ok(Todo { id: row.get(0)?, text: row.get(1)?, completed: row.get(2)?}))?;

    let mut todos = Vec::new();
    for r in rows {
        todos.push(r?);
    }
    Ok(todos)
}
