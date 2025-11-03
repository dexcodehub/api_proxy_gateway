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

#[cfg(test)]
mod tests {
    use super::Pagination;

    #[test]
    fn normalize_clamps_zero_to_defaults() {
        let (idx, per) = Pagination { page: 0, per_page: 0 }.normalize();
        assert_eq!(idx, 0);
        assert_eq!(per, 1);
    }

    #[test]
    fn normalize_clamps_upper_bound() {
        let (idx, per) = Pagination { page: 5, per_page: 1000 }.normalize();
        assert_eq!(idx, 4);
        assert_eq!(per, 100);
    }

    #[test]
    fn default_values_are_sane() {
        let d = Pagination::default();
        assert_eq!(d.page, 1);
        assert_eq!(d.per_page, 20);
    }
}