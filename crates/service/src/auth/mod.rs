//! Auth module: three-layer architecture (domain, repository, service).
//!
//! This module centralizes registration and login business logic under the service crate.

pub mod domain;
pub mod errors;
pub mod repository;
pub mod service;
pub mod repo;

pub use service::AuthService;