use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct MascotSkinCache<T> {
    capacity: usize,
    entries: HashMap<PathBuf, T>,
    order: VecDeque<PathBuf>,
}

impl<T> MascotSkinCache<T> {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity: capacity.max(1),
            entries: HashMap::new(),
            order: VecDeque::new(),
        }
    }

    pub fn get(&self, path: &Path) -> Option<&T> {
        self.entries.get(path)
    }

    pub fn insert(&mut self, path: PathBuf, value: T) {
        if self.entries.contains_key(&path) {
            self.order.retain(|existing| existing != &path);
        }

        self.order.push_back(path.clone());
        self.entries.insert(path, value);

        while self.entries.len() > self.capacity {
            if let Some(evicted_path) = self.order.pop_front() {
                self.entries.remove(&evicted_path);
            }
        }
    }
}
