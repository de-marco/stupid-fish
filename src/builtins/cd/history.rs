extern crate alloc;

use {
    core::time::Duration,
    alloc::{
        collections::BTreeMap,
        sync::Arc,
    },
    std::{
        sync::{LazyLock, RwLock, TryLockError},
        thread,
        time::SystemTime,
    },
};

#[cfg(test)]
mod tests;

#[derive(Debug, Clone)]
pub struct History {
    // path → last visit time
    by_path: BTreeMap<Arc<String>, SystemTime>,
    // time → path   (BTreeMap sorts by time ascending → oldest first)
    by_time: BTreeMap<SystemTime, Arc<String>>,
    max_size: usize,
}

impl History {

    pub const GLOBAL: LazyLock<Arc<RwLock<Self>>> = LazyLock::new(|| Arc::new(RwLock::new(Self::new(99))));

    fn new(max_size: usize) -> Self {
        Self {
            by_path: BTreeMap::new(),
            by_time: BTreeMap::new(),
            max_size: max_size.max(1),
        }
    }

    pub fn add<S>(&mut self, path: S) where S: Into<String> {
        let now = SystemTime::now();
        let path = Arc::new(path.into());

        // 1. Remove old entry if it exists
        if let Some(old_time) = self.by_path.remove(&path) {
            self.by_time.remove(&old_time);
        }

        // 2. Insert new entry
        self.by_path.insert(path.clone(), now);
        self.by_time.insert(now, path);

        // 3. Enforce size limit: remove oldest if needed
        while self.by_time.len() > self.max_size {
            if let Some((_, path)) = self.by_time.pop_first() {
                self.by_path.remove(&path);
            }
        }
    }

    /// Returns iterator over entries in **most recent first** order
    pub fn recents(&self) -> impl Iterator<Item = (&str, SystemTime)> + '_ {
        self.by_time.iter().rev().map(|(time, path)| (path.as_str(), *time))
    }

    pub fn len(&self) -> usize {
        self.by_time.len()
    }

}

pub (super) fn add<S>(path: S) where S: Into<String> {
    loop {
        match History::GLOBAL.try_write() {
            Ok(mut history) => {
                history.add(path);
                return;
            },
            Err(TryLockError::Poisoned(_)) => History::GLOBAL.clear_poison(),
            Err(TryLockError::WouldBlock) => thread::sleep(Duration::from_millis(10)),
        };
    }
}
