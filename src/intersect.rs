use std::collections::HashMap;
use std::hash::Hash;
use std::vec::IntoIter;

use itertools::Itertools;

pub trait HasKey<K> {
    fn has_key(&self, k: &K) -> bool;
}

impl<K, V> HasKey<K> for HashMap<K, V>
where
    K: Eq + Hash,
{
    fn has_key(&self, k: &K) -> bool {
        self.contains_key(k)
    }
}

pub fn filter_vec<'a, const N: usize, K, V, F>(
    keys: Vec<(&'a K, V)>,
    maps: [&dyn HasKey<K>; N],
    filter: F,
) -> Vec<(&'a K, V)>
where
    K: Hash + Eq,
    F: Fn([bool; N]) -> bool,
{
    let maps = maps.each_ref();
    keys.into_iter()
        .filter(|(k, _)| filter(maps.map(|map| map.has_key(k))))
        .collect()
}

pub fn intersect_impl<'a, K, V1, V2, V3, F>(
    mut it1: IntoIter<(K, V1)>,
    mut it2: IntoIter<(K, V2)>,
    f: F,
) -> Vec<(K, V3)>
where
    K: Ord + Eq + Hash,
    V3: 'a,
    F: Fn(V1, V2) -> V3,
{
    let mut res = Vec::new();
    let mut kv1 = it1.next();
    let mut kv2 = it2.next();
    while let (Some((k1, v1)), Some((k2, v2))) = (kv1, kv2) {
        (kv1, kv2) = if k1 < k2 {
            (it1.next(), Some((k2, v2)))
        } else if k2 < k1 {
            (Some((k1, v1)), it2.next())
        } else {
            res.push((k1, f(v1, v2)));
            (it1.next(), it2.next())
        }
    }
    res
}

pub fn intersect<'a, K, V1, V2, V3, F>(
    hm1: Vec<(&'a K, V1)>,
    hm2: &'a HashMap<K, V2>,
    f: F,
) -> Vec<(&'a K, V3)>
where
    K: Ord + Eq + Hash,
    V3: 'a,
    F: Fn(V1, &'a V2) -> V3,
{
    intersect_impl(
        hm1.into_iter().sorted_by_key(|(k, _)| *k),
        hm2.iter().sorted_by_key(|(k, _)| *k),
        f,
    )
}

pub fn intersect_mut<'a, K, V1, V2, V3, F>(
    hm1: Vec<(&'a K, V1)>,
    hm2: &'a mut HashMap<K, V2>,
    f: F,
) -> Vec<(&'a K, V3)>
where
    K: Ord + Eq + Hash,
    V3: 'a,
    F: Fn(V1, &'a mut V2) -> V3,
{
    intersect_impl(
        hm1.into_iter().sorted_by_key(|(k, _)| *k),
        hm2.iter_mut().sorted_by_key(|(k, _)| *k),
        f,
    )
}
