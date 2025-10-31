use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct Health {
    pub status: &'static str,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Post {
    pub user_id: Option<u32>,
    pub id: Option<u32>,
    pub title: String,
    pub body: String,
}
