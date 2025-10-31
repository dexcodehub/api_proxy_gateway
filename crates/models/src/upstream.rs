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

pub fn validate_base_url(base_url: &str) -> Result<(), errors::ModelError> {
    if !base_url.starts_with("http") {
        Err(errors::ModelError::Validation("invalid base_url".into()))
    } else {
        Ok(())
    }
}

pub async fn create(db: &DatabaseConnection, name: &str, base_url: &str) -> Result<Model, errors::ModelError> {
    validate_base_url(base_url)?;
    let now = Utc::now().into();
    let am = ActiveModel { id: Set(Uuid::new_v4()), name: Set(name.to_string()), base_url: Set(base_url.to_string()), health_url: Set(None), active: Set(true), created_at: Set(now), updated_at: Set(now) };
    am.insert(db).await.map_err(|e| errors::ModelError::Db(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_base_url_rejects_invalid() {
        assert!(matches!(validate_base_url("ftp://example"), Err(errors::ModelError::Validation(_))));
        assert!(matches!(validate_base_url(""), Err(errors::ModelError::Validation(_))));
    }

    #[test]
    fn validate_base_url_accepts_http() {
        assert!(validate_base_url("http://example").is_ok());
        assert!(validate_base_url("https://example").is_ok());
    }
}