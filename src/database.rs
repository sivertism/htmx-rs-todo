//use rusqlite::NO_PARAMS;
use rusqlite::{Connection, Result};
use crate::todo::Todo;

fn get_conn() -> Connection {
    // Create, or connect to a local SQLite database to store the todos
    let conn = Connection::open("todos.db").expect("Failed to open database");

    conn.execute(
        "create table if not exists todos (
          id integer primary key,
          task text not null
        )
        ",
        [],
    ).expect("Failed to open database");

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

pub fn create_todo (text: String) {
    let conn = get_conn();
    match conn.execute("INSERT INTO todos (task) values (?1)", &[&text]) {
        Ok(updated) => {
            println!("{} rows were inserted", updated);
        }
        Err(err) => {
            println!("Update failed: {}", err);
        } ,
    }
}

pub fn get_todos () -> Result<Vec<Todo>>{ 
    let conn = get_conn();
    let mut stmt = conn.prepare(
        "select * from todos;",
        )?;

    let rows = stmt.query_map([], |row| Ok(Todo { id: row.get(0)?, text: row.get(1)? }))?;

    let mut todos = Vec::new();
    for r in rows {
        todos.push(r?);
    }
    Ok(todos)
}
