use sea_orm::{entity::prelude::*, Set, DatabaseConnection};
use uuid::Uuid;
use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::errors;
use crate::user;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "api_key")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub user_id: Uuid,
    pub key_hash: String,
    pub status: String,
    pub created_at: DateTimeWithTimeZone,
    pub last_used_at: Option<DateTimeWithTimeZone>,
}

#[derive(Copy, Clone, Debug, EnumIter)]
pub enum Relation { User }

impl RelationTrait for Relation {
    fn def(&self) -> RelationDef {
        match self { Relation::User => Entity::belongs_to(user::Entity).from(Column::UserId).to(user::Column::Id).into() }
    }
}

impl ActiveModelBehavior for ActiveModel {}

pub async fn create(db: &DatabaseConnection, user_id: Uuid, key_hash: &str) -> Result<Model, errors::ModelError> {
    if key_hash.len() < 12 { return Err(errors::ModelError::Validation("key_hash too short".into())); }
    let am = ActiveModel {
        id: Set(Uuid::new_v4()),
        user_id: Set(user_id),
        key_hash: Set(key_hash.to_string()),
        status: Set("active".into()),
        created_at: Set(Utc::now().into()),
        last_used_at: Set(None),
    };
    am.insert(db).await.map_err(|e| errors::ModelError::Db(e.to_string()))
}