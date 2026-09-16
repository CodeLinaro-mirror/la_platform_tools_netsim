// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;

pub fn evict_lru<K, V, F>(cache: &mut HashMap<K, V>, max_entries: usize, key: &K, get_tick: F)
where
    K: std::cmp::Eq + std::hash::Hash + Copy,
    F: Fn(&V) -> u64,
{
    if !cache.contains_key(key) && cache.len() >= max_entries {
        let evict_key = cache.iter().min_by_key(|(_, v)| get_tick(v)).map(|(&k, _)| k);
        if let Some(k) = evict_key {
            cache.remove(&k);
        }
    }
}
