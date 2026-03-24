//! Smart Menu Invalidation
//!
//! Prevents unnecessary menu rebuilds during navigation.
//! Uses version tracking to efficiently detect when menus need updating.

#![allow(dead_code)]

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::RwLock;
use std::time::{Duration, Instant};

/// Global menu content version
static MENU_CONTENT_VERSION: AtomicU64 = AtomicU64::new(0);

/// Delay before refreshing data after menu opens
pub const MENU_OPEN_REFRESH_DELAY: Duration = Duration::from_millis(1200);

/// Menu invalidation tracker
pub struct MenuInvalidationTracker {
    /// Version when each menu was last built
    menu_versions: RwLock<HashMap<String, u64>>,
    /// Currently open menus (prevents invalidation during navigation)
    open_menus: RwLock<Vec<String>>,
    /// Last invalidation time
    last_invalidation: RwLock<Option<Instant>>,
    /// Pending invalidation (deferred because menus are open)
    pending_invalidation: AtomicU64,
}

impl MenuInvalidationTracker {
    /// Create a new tracker
    pub fn new() -> Self {
        Self {
            menu_versions: RwLock::new(HashMap::new()),
            open_menus: RwLock::new(Vec::new()),
            last_invalidation: RwLock::new(None),
            pending_invalidation: AtomicU64::new(0),
        }
    }

    /// Invalidate all menus (increment global version)
    /// Returns true if invalidation was applied, false if deferred
    pub fn invalidate(&self) -> bool {
        // Check if any menus are open
        let open = self.open_menus.read().unwrap();
        if !open.is_empty() {
            // Defer invalidation until menus close
            self.pending_invalidation.fetch_add(1, Ordering::SeqCst);
            tracing::debug!(
                "Menu invalidation deferred (open menus: {})",
                open.len()
            );
            return false;
        }
        drop(open);

        self.apply_invalidation()
    }

    /// Force invalidation even if menus are open (use sparingly)
    pub fn force_invalidate(&self) -> bool {
        self.apply_invalidation()
    }

    /// Apply the invalidation
    fn apply_invalidation(&self) -> bool {
        let new_version = MENU_CONTENT_VERSION.fetch_add(1, Ordering::SeqCst) + 1;
        *self.last_invalidation.write().unwrap() = Some(Instant::now());
        self.pending_invalidation.store(0, Ordering::SeqCst);
        tracing::debug!("Menu content version updated to {}", new_version);
        true
    }

    /// Check if a menu needs refresh
    pub fn needs_refresh(&self, menu_id: &str) -> bool {
        let current_version = MENU_CONTENT_VERSION.load(Ordering::SeqCst);
        let versions = self.menu_versions.read().unwrap();
        match versions.get(menu_id) {
            Some(&version) => version < current_version,
            None => true, // Never built
        }
    }

    /// Mark a menu as freshly built
    pub fn mark_fresh(&self, menu_id: &str) {
        let current_version = MENU_CONTENT_VERSION.load(Ordering::SeqCst);
        let mut versions = self.menu_versions.write().unwrap();
        versions.insert(menu_id.to_string(), current_version);
    }

    /// Called when a menu opens
    pub fn menu_opened(&self, menu_id: &str) {
        let mut open = self.open_menus.write().unwrap();
        if !open.contains(&menu_id.to_string()) {
            open.push(menu_id.to_string());
        }
    }

    /// Called when a menu closes
    pub fn menu_closed(&self, menu_id: &str) {
        let mut open = self.open_menus.write().unwrap();
        open.retain(|id| id != menu_id);

        // Check for pending invalidation
        if open.is_empty() {
            let pending = self.pending_invalidation.load(Ordering::SeqCst);
            if pending > 0 {
                drop(open);
                tracing::debug!("Applying {} deferred invalidation(s)", pending);
                self.apply_invalidation();
            }
        }
    }

    /// Check if any menus are open
    pub fn has_open_menus(&self) -> bool {
        !self.open_menus.read().unwrap().is_empty()
    }

    /// Get current version
    pub fn current_version(&self) -> u64 {
        MENU_CONTENT_VERSION.load(Ordering::SeqCst)
    }

    /// Get pending invalidation count
    pub fn pending_count(&self) -> u64 {
        self.pending_invalidation.load(Ordering::SeqCst)
    }

    /// Clear all tracked versions (useful when completely rebuilding)
    pub fn clear(&self) {
        self.menu_versions.write().unwrap().clear();
    }
}

impl Default for MenuInvalidationTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// Menu section identifier for partial updates
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum MenuSection {
    /// Provider switcher (rarely changes)
    Switcher,
    /// Usage metrics (changes frequently)
    Usage,
    /// Actions (preferences, quit, etc.)
    Actions,
    /// Status/error messages
    Status,
    /// Custom section
    Custom(String),
}

/// Tracks which sections of a menu need updating
#[derive(Debug, Clone, Default)]
pub struct MenuDirtyState {
    /// Dirty sections
    dirty_sections: Vec<MenuSection>,
    /// Whether the entire menu needs rebuild
    full_rebuild_needed: bool,
}

impl MenuDirtyState {
    /// Create a new clean state
    pub fn clean() -> Self {
        Self {
            dirty_sections: Vec::new(),
            full_rebuild_needed: false,
        }
    }

    /// Create a state needing full rebuild
    pub fn full_rebuild() -> Self {
        Self {
            dirty_sections: Vec::new(),
            full_rebuild_needed: true,
        }
    }

    /// Mark a section as dirty
    pub fn mark_dirty(&mut self, section: MenuSection) {
        if !self.dirty_sections.contains(&section) {
            self.dirty_sections.push(section);
        }
    }

    /// Mark as needing full rebuild
    pub fn mark_full_rebuild(&mut self) {
        self.full_rebuild_needed = true;
    }

    /// Check if a section is dirty
    pub fn is_dirty(&self, section: &MenuSection) -> bool {
        self.full_rebuild_needed || self.dirty_sections.contains(section)
    }

    /// Check if any section is dirty
    pub fn has_dirty_sections(&self) -> bool {
        self.full_rebuild_needed || !self.dirty_sections.is_empty()
    }

    /// Check if full rebuild is needed
    pub fn needs_full_rebuild(&self) -> bool {
        self.full_rebuild_needed
    }

    /// Get dirty sections
    pub fn dirty_sections(&self) -> &[MenuSection] {
        &self.dirty_sections
    }

    /// Reset to clean state
    pub fn clear(&mut self) {
        self.dirty_sections.clear();
        self.full_rebuild_needed = false;
    }
}

/// Data staleness checker
pub struct StalenessChecker {
    /// How long data is considered fresh
    freshness_duration: Duration,
    /// Last update times by key
    update_times: RwLock<HashMap<String, Instant>>,
}

impl StalenessChecker {
    /// Create a new staleness checker
    pub fn new(freshness_duration: Duration) -> Self {
        Self {
            freshness_duration,
            update_times: RwLock::new(HashMap::new()),
        }
    }

    /// Create with default freshness (30 seconds)
    pub fn with_default_freshness() -> Self {
        Self::new(Duration::from_secs(30))
    }

    /// Mark data as fresh
    pub fn mark_fresh(&self, key: &str) {
        self.update_times
            .write()
            .unwrap()
            .insert(key.to_string(), Instant::now());
    }

    /// Check if data is stale
    pub fn is_stale(&self, key: &str) -> bool {
        let times = self.update_times.read().unwrap();
        match times.get(key) {
            Some(time) => time.elapsed() > self.freshness_duration,
            None => true, // Never updated = stale
        }
    }

    /// Get time since last update
    pub fn time_since_update(&self, key: &str) -> Option<Duration> {
        self.update_times.read().unwrap().get(key).map(|t| t.elapsed())
    }

    /// Clear all tracked times
    pub fn clear(&self) {
        self.update_times.write().unwrap().clear();
    }
}

impl Default for StalenessChecker {
    fn default() -> Self {
        Self::with_default_freshness()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;

    #[test]
    fn test_tracker_invalidation() {
        let tracker = MenuInvalidationTracker::new();
        let v1 = tracker.current_version();

        tracker.invalidate();
        let v2 = tracker.current_version();
        assert!(v2 > v1);
    }

    #[test]
    fn test_needs_refresh() {
        let tracker = MenuInvalidationTracker::new();

        // New menu always needs refresh
        assert!(tracker.needs_refresh("main"));

        // After marking fresh, doesn't need refresh
        tracker.mark_fresh("main");
        assert!(!tracker.needs_refresh("main"));

        // After invalidation, needs refresh again
        tracker.invalidate();
        assert!(tracker.needs_refresh("main"));
    }

    #[test]
    fn test_deferred_invalidation() {
        let tracker = MenuInvalidationTracker::new();
        tracker.mark_fresh("main");

        // Open a menu
        tracker.menu_opened("main");
        assert!(tracker.has_open_menus());

        // Invalidation should be deferred
        let v1 = tracker.current_version();
        let applied = tracker.invalidate();
        assert!(!applied);
        assert_eq!(tracker.pending_count(), 1);
        assert_eq!(tracker.current_version(), v1);

        // Close menu - pending invalidation should apply
        tracker.menu_closed("main");
        assert!(!tracker.has_open_menus());
        assert!(tracker.current_version() > v1);
        assert_eq!(tracker.pending_count(), 0);
    }

    #[test]
    fn test_dirty_state() {
        let mut state = MenuDirtyState::clean();
        assert!(!state.has_dirty_sections());

        state.mark_dirty(MenuSection::Usage);
        assert!(state.has_dirty_sections());
        assert!(state.is_dirty(&MenuSection::Usage));
        assert!(!state.is_dirty(&MenuSection::Switcher));

        state.clear();
        assert!(!state.has_dirty_sections());
    }

    #[test]
    fn test_full_rebuild_flag() {
        let mut state = MenuDirtyState::clean();
        state.mark_full_rebuild();

        // When full rebuild is needed, all sections are considered dirty
        assert!(state.is_dirty(&MenuSection::Switcher));
        assert!(state.is_dirty(&MenuSection::Usage));
        assert!(state.is_dirty(&MenuSection::Actions));
    }

    #[test]
    fn test_staleness_checker() {
        let checker = StalenessChecker::new(Duration::from_millis(50));

        // Initially stale
        assert!(checker.is_stale("test"));

        // After marking fresh
        checker.mark_fresh("test");
        assert!(!checker.is_stale("test"));

        // After waiting
        sleep(Duration::from_millis(60));
        assert!(checker.is_stale("test"));
    }
}
