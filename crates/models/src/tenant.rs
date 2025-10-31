use sea_orm::{entity::prelude::*, Set, DatabaseConnection};
use uuid::Uuid;
use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::errors;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "tenant")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub name: String,
    pub created_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter)]
pub enum Relation {}

impl RelationTrait for Relation {
    fn def(&self) -> RelationDef { panic!("no relations defined here") }
}

impl ActiveModelBehavior for ActiveModel {}

pub async fn create(db: &DatabaseConnection, name: &str) -> Result<Model, errors::ModelError> {
    if name.trim().is_empty() { return Err(errors::ModelError::Validation("name required".into())); }
    let am = ActiveModel {
        id: Set(Uuid::new_v4()),
        name: Set(name.to_string()),
        created_at: Set(Utc::now().into()),
    };
    am.insert(db).await.map_err(|e| errors::ModelError::Db(e.to_string()))
}