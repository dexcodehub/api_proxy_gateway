//! Pagination utilities for service layer
//!
//! Provides a simple `Pagination` struct and helpers to normalize inputs.

/// Pagination parameters
#[derive(Clone, Copy, Debug)]
pub struct Pagination {
    /// 1-based page index
    pub page: u32,
    /// items per page
    pub per_page: u32,
}

impl Pagination {
    /// Clamp to sane defaults and convert to `u64`
    pub fn normalize(self) -> (u64, u64) {
        let page = if self.page == 0 { 1 } else { self.page };
        let per_page = self.per_page.clamp(1, 100);
        ((page - 1) as u64, per_page as u64)
    }
}

impl Default for Pagination {
    fn default() -> Self { Self { page: 1, per_page: 20 } }
}