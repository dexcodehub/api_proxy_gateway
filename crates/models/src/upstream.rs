use sea_orm::{entity::prelude::*, Set, DatabaseConnection};
use uuid::Uuid;
use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::errors;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "upstream")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub name: String,
    pub base_url: String,
    pub health_url: Option<String>,
    pub active: bool,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter)]
pub enum Relation {}

impl RelationTrait for Relation { fn def(&self) -> RelationDef { panic!("no relations") } }

impl ActiveModelBehavior for ActiveModel {}

pub async fn create(db: &DatabaseConnection, name: &str, base_url: &str) -> Result<Model, errors::ModelError> {
    if !base_url.starts_with("http") { return Err(errors::ModelError::Validation("invalid base_url".into())); }
    let now = Utc::now().into();
    let am = ActiveModel { id: Set(Uuid::new_v4()), name: Set(name.to_string()), base_url: Set(base_url.to_string()), health_url: Set(None), active: Set(true), created_at: Set(now), updated_at: Set(now) };
    am.insert(db).await.map_err(|e| errors::ModelError::Db(e.to_string()))
}