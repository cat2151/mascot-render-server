use std::path::PathBuf;

use crate::MascotSkinCache;

#[test]
fn mascot_skin_cache_reuses_existing_entries() {
    let mut cache = MascotSkinCache::new(2);
    let path = PathBuf::from("cache/demo/a.png");

    cache.insert(path.clone(), 1usize);
    cache.insert(path.clone(), 2usize);

    assert_eq!(cache.get(&path), Some(&2));
}

#[test]
fn mascot_skin_cache_evicts_oldest_entry_when_capacity_is_exceeded() {
    let mut cache = MascotSkinCache::new(2);
    let first = PathBuf::from("cache/demo/a.png");
    let second = PathBuf::from("cache/demo/b.png");
    let third = PathBuf::from("cache/demo/c.png");

    assert!(cache.insert(first.clone(), 1usize).is_empty());
    assert!(cache.insert(second.clone(), 2usize).is_empty());
    let evicted_paths = cache.insert(third.clone(), 3usize);

    assert_eq!(evicted_paths, vec![first.clone()]);
    assert_eq!(cache.get(&first), None);
    assert_eq!(cache.get(&second), Some(&2));
    assert_eq!(cache.get(&third), Some(&3));
}
