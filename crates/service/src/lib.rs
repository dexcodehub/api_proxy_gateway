//! Service layer providing business-oriented CRUD operations on top of models.
//! - Separates business logic from data access.
//! - Reuses validation and entity definitions in `models` crate.
//! - Provides clear error types and documented interfaces.

pub mod errors;
pub mod auth;
pub mod runtime;
pub mod admin_http;
#[cfg(test)]
pub mod test_support;
pub mod storage;
pub mod db;
pub mod file;
pub mod admin;
pub mod proxy_api;
