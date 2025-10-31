use sea_orm::{entity::prelude::*, Set, DatabaseConnection};
use uuid::Uuid;
use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::errors;
use crate::tenant;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "user")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub email: String,
    pub name: String,
    pub status: String,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    pub deleted_at: Option<DateTimeWithTimeZone>,
}

#[derive(Copy, Clone, Debug, EnumIter)]
pub enum Relation {
    Tenant,
}

impl RelationTrait for Relation {
    fn def(&self) -> RelationDef {
        match self { Relation::Tenant => Entity::belongs_to(tenant::Entity).from(Column::TenantId).to(tenant::Column::Id).into() }
    }
}

impl ActiveModelBehavior for ActiveModel {}

pub async fn create(db: &DatabaseConnection, tenant_id: Uuid, email: &str, name: &str) -> Result<Model, errors::ModelError> {
    if !email.contains('@') { return Err(errors::ModelError::Validation("invalid email".into())); }
    if name.trim().is_empty() { return Err(errors::ModelError::Validation("name required".into())); }
    let now = Utc::now().into();
    let am = ActiveModel {
        id: Set(Uuid::new_v4()),
        tenant_id: Set(tenant_id),
        email: Set(email.to_string()),
        name: Set(name.to_string()),
        status: Set("active".into()),
        created_at: Set(now),
        updated_at: Set(now),
        deleted_at: Set(None),
    };
    am.insert(db).await.map_err(|e| errors::ModelError::Db(e.to_string()))
}

pub async fn soft_delete(db: &DatabaseConnection, id: Uuid) -> Result<(), errors::ModelError> {
    let mut found: ActiveModel = Entity::find_by_id(id).one(db).await.map_err(|e| errors::ModelError::Db(e.to_string()))?.ok_or_else(|| errors::ModelError::Validation("user not found".into()))?.into();
    found.deleted_at = Set(Some(Utc::now().into()));
    found.update(db).await.map_err(|e| errors::ModelError::Db(e.to_string()))?;
    Ok(())
}

pub async fn hard_delete(db: &DatabaseConnection, id: Uuid) -> Result<(), errors::ModelError> {
    Entity::delete_by_id(id).exec(db).await.map_err(|e| errors::ModelError::Db(e.to_string()))?;
    Ok(())
}