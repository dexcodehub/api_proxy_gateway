use sea_orm::entity::prelude::*;
use sea_orm::{EntityTrait, ColumnTrait, QueryFilter, ActiveModelTrait, Set, DatabaseConnection};
use uuid::Uuid;
use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::user;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "user_credentials")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub user_id: Uuid,
    pub password_hash: String,
    pub password_algorithm: String,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter)]
pub enum Relation { User }

impl RelationTrait for Relation {
    fn def(&self) -> RelationDef {
        match self {
            Relation::User => Entity::belongs_to(user::Entity)
                .from(Column::UserId)
                .to(user::Column::Id)
                .into(),
        }
    }
}

impl ActiveModelBehavior for ActiveModel {}

pub async fn upsert_password(
    db: &DatabaseConnection,
    user_id: Uuid,
    password_hash: String,
    algorithm: &str,
) -> Result<Model, crate::errors::ModelError> {
    if password_hash.trim().is_empty() {
        return Err(crate::errors::ModelError::Validation("password hash required".into()));
    }
    let now = Utc::now().into();
    if let Some(existing) = Entity::find()
        .filter(Column::UserId.eq(user_id))
        .one(db)
        .await
        .map_err(|e| crate::errors::ModelError::Db(e.to_string()))? {
        let mut am: ActiveModel = existing.into();
        am.password_hash = Set(password_hash);
        am.password_algorithm = Set(algorithm.to_string());
        am.updated_at = Set(now);
        let updated = am
            .update(db)
            .await
            .map_err(|e| crate::errors::ModelError::Db(e.to_string()))?;
        Ok(updated)
    } else {
        let am = ActiveModel {
            id: Set(Uuid::new_v4()),
            user_id: Set(user_id),
            password_hash: Set(password_hash),
            password_algorithm: Set(algorithm.to_string()),
            created_at: Set(now),
            updated_at: Set(now),
        };
        let created = am
            .insert(db)
            .await
            .map_err(|e| crate::errors::ModelError::Db(e.to_string()))?;
        Ok(created)
    }
}

pub async fn verify_password(
    db: &DatabaseConnection,
    user_id: Uuid,
    verify_fn: impl Fn(&str) -> bool,
) -> Result<bool, crate::errors::ModelError> {
    let creds = Entity::find()
        .filter(Column::UserId.eq(user_id))
        .one(db)
        .await
        .map_err(|e| crate::errors::ModelError::Db(e.to_string()))?;
    Ok(match creds { Some(c) => verify_fn(&c.password_hash), None => false })
}