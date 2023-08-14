// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT

use num_traits::{PrimInt};
use std::cmp::{max, min};
use std::collections::HashMap;
use std::hash::Hash;

type Bounds<T> = (T, T);

/// The key value cache. The key must be numeric and falls within a range/bound.
pub struct RangeKeyCache<Key, Value> {
    /// Stores the data in a hashmap.
    data: HashMap<Key, Value>,
    /// The lower and upper bound of keys stored in data
    bounds: Option<Bounds<Key>>,
}

impl<Key: PrimInt + Hash, Value> RangeKeyCache<Key, Value> {
    pub fn new() -> Self {
        Self {
            data: Default::default(),
            bounds: None,
        }
    }

    pub fn upper_bound(&self) -> Option<Key> {
        self.within_bounds(|(_, upper)| *upper)
    }

    pub fn get_value(&self, key: Key) -> Option<&Value> {
        self.within_bounds(|(lower_bound, upper_bound)| {
            if *lower_bound > key || *upper_bound < key {
                return None;
            }
            return self.data.get(&key);
        })
        .flatten()
    }

    pub fn values_within_range(&self, start: Key, end: Option<Key>) -> Vec<&Value> {
        self.within_bounds(|(lower_bound, upper_bound)| {
            let start = max(*lower_bound, start);
            let end = min(*upper_bound, end.unwrap_or(*upper_bound));

            let mut r = vec![];
            let mut i = start;
            while i <= end {
                if let Some(v) = self.get_value(i) {
                    r.push(v);
                }

                i = i + Key::one();
            }

            r
        })
        .unwrap_or(vec![])
    }

    /// Removes the block hashes stored till the specified height, exclusive.
    pub fn remove_key_till(&mut self, key: Key) {
        if let Some((lower_bound, upper_bound)) = self.bounds.as_mut() {
            if *lower_bound > key || *upper_bound < key {
                return;
            }

            let mut i = *lower_bound;
            while i < key {
                self.data.remove(&i);
                i = i + Key::one();
            }

            *lower_bound = key;
        }
    }

    /// Insert the block hash at the next height
    pub fn insert_after_lower_bound(&mut self, key: Key, val: Value) {
        match &mut self.bounds {
            None => {
                self.data.insert(key, val);
                self.bounds.replace((key, key));
            }
            Some((upper, lower)) => {
                if *lower > key {
                    return;
                }

                self.data.insert(key, val);
                if *upper < key {
                    *upper = key;
                }
            }
        }
    }

    fn within_bounds<F, T>(&self, f: F) -> Option<T>
    where
        F: Fn(&(Key, Key)) -> T,
    {
        self.bounds.as_ref().map(f)
    }
}
