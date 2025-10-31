use sea_orm::entity::prelude::*;
use uuid::Uuid;
use serde::{Deserialize, Serialize};

use crate::{route, apikey};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "request_log")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub route_id: Uuid,
    pub api_key_id: Option<Uuid>,
    pub status_code: i32,
    pub latency_ms: i32,
    pub success: bool,
    pub error_message: Option<String>,
    pub client_ip: Option<String>,
    pub timestamp: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter)]
pub enum Relation { Route, ApiKey }

impl RelationTrait for Relation {
    fn def(&self) -> RelationDef {
        match self {
            Relation::Route => Entity::belongs_to(route::Entity).from(Column::RouteId).to(route::Column::Id).into(),
            Relation::ApiKey => Entity::belongs_to(apikey::Entity).from(Column::ApiKeyId).to(apikey::Column::Id).into(),
        }
    }
}

impl ActiveModelBehavior for ActiveModel {}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;
    use chrono::Utc;

    #[test]
    fn construct_model() {
        let m = Model {
            id: 1,
            route_id: Uuid::new_v4(),
            api_key_id: None,
            status_code: 200,
            latency_ms: 123,
            success: true,
            error_message: None,
            client_ip: Some("127.0.0.1".into()),
            timestamp: Utc::now().into(),
        };
        assert_eq!(m.status_code, 200);
        assert!(m.success);
    }
}