//! Storage abstractions for service layer
//!
//! Contains reusable file-backed stores and helpers to avoid duplication
//! across services that persist small maps as JSON.

pub mod json_map_store;