//! Service layer providing business-oriented CRUD operations on top of models.
//! - Separates business logic from data access.
//! - Reuses validation and entity definitions in `models` crate.
//! - Provides clear error types and documented interfaces.

pub mod errors;
pub mod user_service;
pub mod tenant_service;
pub mod apikey_service;
pub mod upstream_service;
pub mod route_service;
pub mod ratelimit_service;
pub mod request_log_service;
pub mod pagination;
pub mod runtime;
pub mod admin_http;
#[cfg(test)]
pub mod test_support;
