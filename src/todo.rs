use serde::Deserialize;

#[derive(Clone, Debug)]
pub struct Task {
    pub id: usize,
    pub text: String,
    pub completed: bool,
}

// Don't worry about this for now, will use later
#[derive(Deserialize)]
pub struct TaskForm {
    pub text: String,
}

#[derive(Deserialize)]
pub struct ListForm {
    pub text: String,
}
