use serde::Deserialize;

#[derive(Clone, Debug)]
pub struct Todo {
    pub id: usize,
    pub text: String,
    pub completed: bool,
}

// Don't worry about this for now, will use later
#[derive(Deserialize)]
pub struct TodoForm {
    pub text: String,
}
