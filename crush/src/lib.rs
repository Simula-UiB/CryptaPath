#[macro_use]
extern crate nom;
#[macro_use]
#[cfg(test)]
extern crate vob;

#[macro_use]
pub mod algebra;
pub mod soc;
pub mod solver;

use core::hash::BuildHasherDefault;
use std::collections::{HashMap, HashSet};
type AHashMap<K, V> = HashMap<K, V, BuildHasherDefault<ahash::AHasher>>;
type AHashSet<K> = HashSet<K, BuildHasherDefault<ahash::AHasher>>;
