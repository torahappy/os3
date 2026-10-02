use std::{cmp::Reverse, collections::BinaryHeap};

use bevy::platform::collections::HashMap;

pub fn most_frequent<T>(array: &[T], k: usize) -> Vec<(usize, &T)>
where
    T: std::hash::Hash + Eq + Ord,
{
    let mut map = HashMap::new();
    for x in array {
        *map.entry(x).or_default() += 1;
    }

    let mut heap = BinaryHeap::with_capacity(k + 1);
    for (x, count) in map.into_iter() {
        heap.push(Reverse((count, x)));
        if heap.len() > k {
            heap.pop();
        }
    }
    heap.into_sorted_vec().into_iter().map(|r| r.0).collect()
}
