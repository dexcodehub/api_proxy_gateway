use utoipa::OpenApi;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(ToSchema)]
pub struct HealthResponse { pub status: String }

#[derive(utoipa::ToSchema)]
pub struct RegisterRequest { pub tenant_id: Uuid, pub email: String, pub name: String, pub password: String }

#[derive(utoipa::ToSchema)]
pub struct LoginRequest { pub tenant_id: Uuid, pub email: String, pub password: String }

#[derive(utoipa::ToSchema)]
pub struct ApiKeyRecordDoc { pub user: String, pub api_key: String }

#[derive(utoipa::ToSchema)]
pub struct CreateProxyApiInputDoc {
    pub tenant_id: Option<String>,
    pub endpoint_url: String,
    pub method: String,
    pub forward_target: String,
    pub require_api_key: bool,
}

#[derive(utoipa::ToSchema)]
pub struct UpdateProxyApiInputDoc {
    pub endpoint_url: Option<String>,
    pub method: Option<String>,
    pub forward_target: Option<String>,
    pub require_api_key: Option<bool>,
    pub enabled: Option<bool>,
}

#[derive(OpenApi)]
#[openapi(
    paths(
        crate::routes::health,
        crate::routes::auth::register,
        crate::routes::auth::login,
        crate::routes::admin::list_api_keys,
        crate::routes::admin::set_api_key,
        crate::routes::proxy_apis::list,
        crate::routes::proxy_apis::create,
        crate::routes::proxy_apis::get,
        crate::routes::proxy_apis::update,
        crate::routes::proxy_apis::delete,
    ),
    components(
        schemas(
            HealthResponse,
            RegisterRequest,
            LoginRequest,
            ApiKeyRecordDoc,
            CreateProxyApiInputDoc,
            UpdateProxyApiInputDoc,
        )
    ),
    tags(
        (name = "health"),
        (name = "auth"),
        (name = "admin"),
        (name = "proxy")
    )
)]
pub struct ApiDoc;