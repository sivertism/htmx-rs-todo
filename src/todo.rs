use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct Todo {
    pub id: usize,
    pub text: String,
}

// Don't worry about this for now, will use later
#[derive(Deserialize)]
pub struct TodoForm {
    pub text: String,
}
