// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT

use num_traits::PrimInt;
use std::collections::VecDeque;
use std::fmt::Debug;

/// The key value cache such that:
/// 1. Key must be numeric
/// 2. Keys must be sequential
pub(crate) struct SequentialKeyCache<K, V> {
    /// The underlying data
    data: VecDeque<(K, V)>,
}

/// The result enum for sequential cache insertion
pub(crate) enum SequentialCacheInsert {
    Ok,
    AboveBound,
    /// Not the next expect key value
    NotNext,
    BelowBound,
}

impl<K: PrimInt + Debug, V> SequentialKeyCache<K, V> {
    pub fn new() -> Self {
        Self {
            data: Default::default(),
        }
    }

    pub fn upper_bound(&self) -> Option<K> {
        self.data.back().map(|v| v.0)
    }

    pub fn lower_bound(&self) -> Option<K> {
        self.data.front().map(|v| v.0)
    }

    fn within_bound(&self, k: K) -> bool {
        match (self.lower_bound(), self.upper_bound()) {
            (Some(lower), Some(upper)) => lower <= k && k <= upper,
            (None, None) => false,
            // other states are not reachable, even if there is one entry, both upper and
            // lower bounds should be the same, both should be Some.
            _ => unreachable!(),
        }
    }

    pub fn get_value(&self, key: K) -> Option<&V> {
        if !self.within_bound(key) {
            return None;
        }

        let lower = self.lower_bound().unwrap();
        // safe to unwrap as index must be uint
        let index = (key - lower).to_usize().unwrap();

        self.data.get(index).map(|entry| &entry.1)
    }

    pub fn values_from(&self, start: K) -> Vec<&V> {
        if !self.within_bound(start) {
            return vec![];
        }

        let lower = self.lower_bound().unwrap();
        // safe to unwrap as index must be uint
        let index = (start - lower).to_usize().unwrap();

        let mut results = vec![];
        for i in index..self.data.len() {
            results.push(&self.data.get(i).unwrap().1);
        }

        results
    }

    pub fn values(&self) -> Vec<&V> {
        self.data.iter().map(|i| &i.1).collect()
    }

    /// Removes the all the keys below the target value, exclusive.
    pub fn remove_key_below(&mut self, key: K) {
        while let Some((k, _)) = self.data.front() {
            if *k < key {
                self.data.pop_front();
                continue;
            }
            break;
        }
    }

    /// Removes the all the keys above the target value, exclusive.
    pub fn remove_key_above(&mut self, key: K) {
        while let Some((k, _)) = self.data.back() {
            if *k > key {
                self.data.pop_back();
                continue;
            }
            break;
        }
    }

    /// Insert the key and value pair only if the key is upper_bound + 1
    pub fn insert(&mut self, key: K, val: V) -> SequentialCacheInsert {
        if let Some(upper) = self.upper_bound() {
            if upper.add(K::one()) == key {
                self.data.push_back((key, val));
                return SequentialCacheInsert::Ok;
            } else if upper < key {
                tracing::debug!("key: {key:?} greater than upper bound: {upper:?}");
                return SequentialCacheInsert::AboveBound;
            }

            let lower = self.lower_bound().unwrap();
            if key < lower {
                return SequentialCacheInsert::BelowBound;
            }
            return SequentialCacheInsert::NotNext;
        }

        self.data.push_back((key, val));
        SequentialCacheInsert::Ok
    }
}
