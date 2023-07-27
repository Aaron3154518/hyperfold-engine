use std::hash::Hash;

use itertools::Itertools;

pub fn intersect<'a, K, V1, V2, V3, F>(
    it1: impl IntoIterator<Item = (K, V1)>,
    it2: impl IntoIterator<Item = (K, V2)>,
    f: F,
) -> Vec<(K, V3)>
where
    K: Ord + Eq + Hash,
    V3: 'a,
    F: Fn(V1, V2) -> V3,
{
    let mut it1 = it1.into_iter().sorted_by(|(k1, _), (k2, _)| k1.cmp(k2));
    let mut it2 = it2.into_iter().sorted_by(|(k1, _), (k2, _)| k1.cmp(k2));
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

pub fn intersect_opt<'a, K, V1, V2, V3, F>(
    it1: impl ExactSizeIterator<Item = (K, V1)>,
    it2: impl ExactSizeIterator<Item = (K, V2)>,
    f: F,
) -> Vec<(K, V3)>
where
    K: Ord + Eq + Hash,
    V3: 'a,
    F: Fn(V1, Option<V2>) -> V3,
{
    let mut res = Vec::with_capacity(it1.len());
    let mut it1 = it1.into_iter().sorted_by(|(k1, _), (k2, _)| k1.cmp(k2));
    let mut it2 = it2.into_iter().sorted_by(|(k1, _), (k2, _)| k1.cmp(k2));
    let mut kv2 = it2.next();
    while let Some((k1, v1)) = it1.next() {
        while kv2.as_ref().is_some_and(|(k2, _)| k2 < &k1) {
            kv2 = it2.next();
        }

        if kv2.as_ref().is_some_and(|(k2, _)| &k1 < k2) {
            res.push((k1, f(v1, None)));
            continue;
        }

        match kv2 {
            // k2 == k1
            Some((_, v2)) => {
                res.push((k1, f(v1, Some(v2))));
                kv2 = it2.next();
            }
            None => {
                res.push((k1, f(v1, None)));
                res.extend(it1.map(|(k1, v1)| (k1, f(v1, None))));
                return res;
            }
        }
    }
    res
}
