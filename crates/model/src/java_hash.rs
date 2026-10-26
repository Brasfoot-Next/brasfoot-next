//! `java.util.HashMap` iteration order. The mod writes some of its side files by walking a
//! `HashMap` (`values()`), and the order decides ties when the lists are sorted afterwards, so the
//! ports keep their maps in a [`JavaHashMap`], which iterates like the Java one.

use std::collections::HashMap;
use std::hash::Hash;

/// `Object.hashCode()` of the Java key type.
pub trait JavaHash {
    fn java_hash(&self) -> i32;
}

/// `String.hashCode()`: over the UTF-16 code units.
impl JavaHash for str {
    fn java_hash(&self) -> i32 {
        self.encode_utf16().fold(0i32, |h, unit| h.wrapping_mul(31).wrapping_add(unit as i32))
    }
}

impl JavaHash for String {
    fn java_hash(&self) -> i32 {
        self.as_str().java_hash()
    }
}

/// `Integer.hashCode()`
impl JavaHash for i32 {
    fn java_hash(&self) -> i32 {
        *self
    }
}

/// `HashMap.DEFAULT_INITIAL_CAPACITY`
const INITIAL_CAPACITY: usize = 16;
/// `HashMap.TREEIFY_THRESHOLD`
const TREEIFY_THRESHOLD: usize = 8;
/// `HashMap.MIN_TREEIFY_CAPACITY`
const MIN_TREEIFY_CAPACITY: usize = 64;

/// `HashMap.hash(Object)`
fn spread(hash: i32) -> u32 {
    let h = hash as u32;
    h ^ (h >> 16)
}

/// A `java.util.HashMap` built with `new HashMap<>()` and only `put`s (no removals): lookups by
/// key, values in the order the Java map iterates them. Within a bucket the Java map keeps the
/// insertion order and a resize keeps it too, so the order is the bucket index at the final
/// capacity, then insertion. A bucket of 9 or more keys at a capacity of 64 or more becomes a tree
/// whose later insertions go after their tree parent; that needs 9 colliding hashes and is not
/// reproduced (such keys keep their insertion order).
#[derive(Clone, Debug)]
pub struct JavaHashMap<K, V> {
    entries: Vec<(K, V)>,
    hashes: Vec<u32>,
    index: HashMap<K, usize>,
    capacity: usize,
    /// Keys per bucket at the current capacity, for the resize of `treeifyBin`.
    bins: Vec<usize>,
}

impl<K: JavaHash + Hash + Eq + Clone, V> Default for JavaHashMap<K, V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K: JavaHash + Hash + Eq + Clone, V> JavaHashMap<K, V> {
    pub fn new() -> Self {
        JavaHashMap { entries: Vec::new(), hashes: Vec::new(), index: HashMap::new(), capacity: INITIAL_CAPACITY, bins: vec![0; INITIAL_CAPACITY] }
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn contains_key(&self, key: &K) -> bool {
        self.index.contains_key(key)
    }

    pub fn get(&self, key: &K) -> Option<&V> {
        self.index.get(key).map(|&i| &self.entries[i].1)
    }

    pub fn get_mut(&mut self, key: &K) -> Option<&mut V> {
        self.index.get(key).map(|&i| &mut self.entries[i].1)
    }

    /// `put`: a key already there keeps its place and takes the new value.
    pub fn insert(&mut self, key: K, value: V) {
        if let Some(&i) = self.index.get(&key) {
            self.entries[i].1 = value;
            return;
        }
        self.add(key, value);
    }

    /// `computeIfAbsent`
    pub fn entry_or_insert_with(&mut self, key: K, value: impl FnOnce() -> V) -> &mut V {
        let i = match self.index.get(&key) {
            Some(&i) => i,
            None => self.add(key, value()),
        };
        &mut self.entries[i].1
    }

    /// `putVal` for a new key: appended to its bucket; a bucket that had 8 keys resizes a table
    /// smaller than 64 (`treeifyBin`), and so does a size above three quarters of the capacity.
    fn add(&mut self, key: K, value: V) -> usize {
        let hash = spread(key.java_hash());
        let bucket = hash as usize & (self.capacity - 1);
        let before = self.bins[bucket];
        self.bins[bucket] += 1;
        self.index.insert(key.clone(), self.entries.len());
        self.entries.push((key, value));
        self.hashes.push(hash);
        if before >= TREEIFY_THRESHOLD && self.capacity < MIN_TREEIFY_CAPACITY {
            self.resize();
        }
        if self.entries.len() > self.capacity / 4 * 3 {
            self.resize();
        }
        self.entries.len() - 1
    }

    fn resize(&mut self) {
        self.capacity *= 2;
        self.bins = vec![0; self.capacity];
        for &h in &self.hashes {
            self.bins[h as usize & (self.capacity - 1)] += 1;
        }
    }

    /// The insertion indices in the Java iteration order.
    fn order(&self) -> Vec<usize> {
        let mut order: Vec<usize> = (0..self.entries.len()).collect();
        order.sort_by_key(|&i| self.hashes[i] as usize & (self.capacity - 1));
        order
    }

    /// `entrySet()` in the Java order.
    pub fn iter(&self) -> impl Iterator<Item = (&K, &V)> {
        self.order().into_iter().map(|i| (&self.entries[i].0, &self.entries[i].1))
    }

    /// `values()` in the Java order.
    pub fn values(&self) -> impl Iterator<Item = &V> {
        self.iter().map(|(_, v)| v)
    }

    /// The values in the Java order, taken out of the map.
    pub fn into_values(self) -> Vec<V> {
        let order = self.order();
        let mut slots: Vec<Option<V>> = self.entries.into_iter().map(|(_, v)| Some(v)).collect();
        order.into_iter().map(|i| slots[i].take().expect("each index once")).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn string_hash_is_java() {
        assert_eq!("".java_hash(), 0);
        assert_eq!("a".java_hash(), 97);
        // "Aa" and "BB" collide in Java
        assert_eq!("Aa".java_hash(), "BB".java_hash());
        assert_eq!("hello".java_hash(), 99_162_322);
        assert_eq!("é".java_hash(), 233);
    }

    #[test]
    fn integers_iterate_by_bucket() {
        let mut m = JavaHashMap::new();
        for k in [17, 3, 1, 33, 2] {
            m.insert(k, k);
        }
        // capacity 16: buckets 1 (17, 1, 33), 2, 3
        assert_eq!(m.values().copied().collect::<Vec<_>>(), vec![17, 1, 33, 2, 3]);
    }

    #[test]
    fn resizes_past_three_quarters() {
        let mut m = JavaHashMap::new();
        for k in 0..13 {
            m.insert(k * 16, k);
        }
        // 13 keys > 12: capacity 32, even multiples of 16 in bucket 0, odd ones in bucket 16
        let order: Vec<i32> = m.values().copied().collect();
        assert_eq!(order, vec![0, 2, 4, 6, 8, 10, 12, 1, 3, 5, 7, 9, 11]);
    }

    #[test]
    fn a_full_bucket_resizes_a_small_table() {
        let mut m = JavaHashMap::new();
        for k in 0..9 {
            m.insert(k * 64, k);
        }
        // the ninth key in bucket 0 doubles the table (treeifyBin below 64), still bucket 0
        assert_eq!(m.capacity, 32);
        assert_eq!(m.values().copied().collect::<Vec<_>>(), (0..9).collect::<Vec<_>>());
    }
}
