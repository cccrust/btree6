//! Page-level lock management (placeholder for future concurrent support)

/// Lock manager - simplified for single-threaded use
#[derive(Clone, Default)]
pub struct LockManager;

impl LockManager {
    pub fn new() -> Self {
        Self
    }

    /// Lock a page (no-op in single-threaded mode)
    #[allow(dead_code)]
    pub fn lock_page(&self, _page_id: u64) -> LockGuard {
        LockGuard
    }

    #[allow(dead_code)]
    pub fn clear(&self) {}
}

/// Guard for page lock
pub struct LockGuard;

impl Drop for LockGuard {
    fn drop(&mut self) {}
}
