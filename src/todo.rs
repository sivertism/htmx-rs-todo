use serde::Deserialize;

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct Task {
    pub id: usize,
    pub text: String,
    pub completed: bool,
    pub list_id: usize,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct List {
    pub id: usize,
    pub name: String,
}

// Don't worry about this for now, will use later
#[allow(dead_code)]
#[derive(Deserialize)]
pub struct TaskForm {
    pub text: String,
}

#[allow(dead_code)]
#[derive(Deserialize)]
pub struct ListForm {
    pub name: String,
    pub grocy_url: Option<String>,
    pub grocy_api_key: Option<String>,
}
