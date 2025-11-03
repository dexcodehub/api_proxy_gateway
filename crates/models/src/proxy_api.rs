use sea_orm::{entity::prelude::*, Set, DatabaseConnection, ActiveModelTrait, EntityTrait};
use uuid::Uuid;
use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::{errors, tenant};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "proxy_api")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub endpoint_url: String,
    pub method: String,
    pub forward_target: String,
    pub require_api_key: bool,
    pub enabled: bool,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter)]
pub enum Relation { Tenant }

impl RelationTrait for Relation {
    fn def(&self) -> RelationDef {
        match self {
            Relation::Tenant => Entity::belongs_to(tenant::Entity)
                .from(Column::TenantId)
                .to(tenant::Column::Id)
                .into(),
        }
    }
}

impl ActiveModelBehavior for ActiveModel {}

pub fn validate_method(m: &str) -> Result<String, errors::ModelError> {
    let up = m.to_ascii_uppercase();
    let valid = ["GET","POST","PUT","DELETE","PATCH","HEAD","OPTIONS"];
    if !valid.contains(&up.as_str()) {
        return Err(errors::ModelError::Validation("invalid HTTP method".into()));
    }
    Ok(up)
}

pub fn validate_endpoint_url(p: &str) -> Result<(), errors::ModelError> {
    if !p.starts_with('/') {
        return Err(errors::ModelError::Validation("endpoint_url must start with '/'".into()));
    }
    Ok(())
}

pub fn validate_forward_target(u: &str) -> Result<(), errors::ModelError> {
    if !(u.starts_with("http://") || u.starts_with("https://")) {
        return Err(errors::ModelError::Validation("forward_target must start with http(s)".into()));
    }
    Ok(())
}

pub async fn create(
    db: &DatabaseConnection,
    tenant_id: Uuid,
    endpoint_url: &str,
    method: &str,
    forward_target: &str,
    require_api_key: bool,
) -> Result<Model, errors::ModelError> {
    validate_endpoint_url(endpoint_url)?;
    let method = validate_method(method)?;
    validate_forward_target(forward_target)?;

    let now = Utc::now().into();
    let am = ActiveModel {
        id: Set(Uuid::new_v4()),
        tenant_id: Set(tenant_id),
        endpoint_url: Set(endpoint_url.to_string()),
        method: Set(method),
        forward_target: Set(forward_target.to_string()),
        require_api_key: Set(require_api_key),
        enabled: Set(true),
        created_at: Set(now),
        updated_at: Set(now),
    };
    am.insert(db).await.map_err(|e| errors::ModelError::Db(e.to_string()))
}

pub async fn set_enabled(db: &DatabaseConnection, id: Uuid, enabled: bool) -> Result<(), errors::ModelError> {
    let mut found: ActiveModel = Entity::find_by_id(id)
        .one(db)
        .await
        .map_err(|e| errors::ModelError::Db(e.to_string()))?
        .ok_or_else(|| errors::ModelError::Validation("proxy_api not found".into()))?
        .into();
    found.enabled = Set(enabled);
    found.updated_at = Set(Utc::now().into());
    found.update(db).await.map_err(|e| errors::ModelError::Db(e.to_string()))?;
    Ok(())
}