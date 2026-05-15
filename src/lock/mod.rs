//! Page-level lock management

/// Page latch placeholder - for future concurrent support
pub struct PageLatch(());

impl PageLatch {
    pub fn new() -> Self {
        Self(())
    }
}

impl Default for PageLatch {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for PageLatch {
    fn clone(&self) -> Self {
        Self(())
    }
}

/// Lock manager - placeholder for concurrent support
#[derive(Clone, Default)]
pub struct LockManager;

impl LockManager {
    pub fn new() -> Self {
        Self
    }

    /// Lock page for reading (no-op in single-threaded mode)
    pub fn read_lock(&self, _page_id: u64) -> LockGuard {
        LockGuard
    }

    /// Lock page for writing (no-op)
    pub fn write_lock(&self, _page_id: u64) -> LockGuard {
        LockGuard
    }

    #[allow(dead_code)]
    pub fn clear(&self) {}

    #[allow(dead_code)]
    pub fn lock_count(&self) -> usize {
        0
    }
}

/// Lock guard (no-op)
pub struct LockGuard;

impl Drop for LockGuard {
    fn drop(&mut self) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lock_manager() {
        let manager = LockManager::new();
        let _guard = manager.read_lock(1);
        let _guard2 = manager.read_lock(1);
    }
}
