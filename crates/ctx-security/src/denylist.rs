//! Path denylist for sensitive files

pub struct Denylist {
    // TODO: Add patterns
}

impl Denylist {
    pub fn new() -> Self {
        // TODO: Implement in M4
        todo!("Implement Denylist::new")
    }

    pub fn is_denied(&self, _path: &str) -> (bool, Option<String>) {
        // TODO: Implement in M4
        // - Check path against denylist patterns
        // - Return (is_denied, matched_pattern)
        todo!("Implement Denylist::is_denied")
    }
}

impl Default for Denylist {
    fn default() -> Self {
        Self::new()
    }
}
