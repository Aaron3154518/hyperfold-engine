use std::collections::{BTreeSet, HashMap, HashSet};
use std::hash::Hash;

pub fn intersect<'a, K, V1, V2, F>(
    mut h: HashMap<&'a K, V1>,
    h_new: &'a HashMap<K, V2>,
    get: F,
) -> HashMap<&'a K, V1>
where
    K: Eq + Hash + Clone + Ord,
    F: Fn(&mut V1) -> &mut Option<&'a V2>,
{
    for (k, v) in h_new.iter() {
        if let Some(v2) = h.get_mut(k) {
            *get(v2) = Some(v)
        }
    }
    h.drain_filter(|_k, v| get(v).is_none());
    h
}

pub fn intersect_mut<'a, K, V1, V2, F>(
    mut h: HashMap<&'a K, V1>,
    h_new: &'a mut HashMap<K, V2>,
    get: F,
) -> HashMap<&'a K, V1>
where
    K: Eq + Hash + Clone + Ord,
    F: Fn(&mut V1) -> &mut Option<&'a mut V2>,
{
    for (k, v) in h_new.iter_mut() {
        if let Some(v2) = h.get_mut(k) {
            *get(v2) = Some(v)
        }
    }
    h.drain_filter(|_k, v| get(v).is_none());
    h
}

pub fn intersect_keys<K>(keys: &mut [HashSet<&K>]) -> BTreeSet<K>
where
    K: Eq + Hash + Clone + Ord,
{
    keys.sort_by(|s1, s2| s1.len().cmp(&s2.len()));
    if let Some(k1) = keys.first() {
        let mut k1 = k1.clone();
        keys[1..]
            .iter()
            .for_each(|k| k1 = k1.intersection(k).map(|k| *k).collect::<HashSet<_>>());
        return k1.iter().map(|k| (*k).clone()).collect();
    }
    BTreeSet::new()
}

pub fn get_keys<'a, K, V>(map: &'a HashMap<K, V>) -> HashSet<&'a K>
where
    K: Eq + Hash + Clone,
{
    map.keys().collect()
}
