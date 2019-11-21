//! This is an implementation of a level of a BDD (Binary Decision Diagram).
//!
//! A level is defined as an lhs (left hand side) and multiple nodes.
//!
//! The lhs is stored as a vector of bits (`Vob`) which can be read as follows :
//!
//! x1 + x3 + x5 in a 7 variables system would be stored as [0101010]
//!
//! The nodes are a stored as a `AHasmap` of `Node` with the `Id` of a node as its key.
//! All ids are supposed to be unique in the entirity of the system. The Hashmap uses
//! AHash as its default hasher for speedup over SipHash.

use crate::soc::{node::Node, Id};
use std::fmt;
extern crate vob;
use crate::{AHashMap, AHashSet};
use std::collections::hash_map::{Iter, IterMut};
use vob::{IterSetBits, Vob};

/// A level inside a Binary Decision Diagram
#[derive(Default)]
pub struct Level {
    nodes: AHashMap<Id, Node>,
    lhs: Vob,
}

impl Level {
    /// Construct a new Level with default parameters
    pub fn new() -> Level {
        Default::default()
    }

    /// Set `lhs` to a `Vob` of size `var_len` with all the bits specified
    /// in `vars` equals to `true`.
    /// ```text.
    /// set_lhs(vec![1,3,4],6) -> lhs = [010110]
    /// ```
    pub fn set_lhs(&mut self, vars: Vec<usize>, var_len: usize) {
        self.lhs.resize(var_len, false);
        for var in vars.iter() {
            self.lhs.set(*var, !self.lhs.get(*var).unwrap());
        }
    }

    /// Return a clone of `lhs`.
    #[inline]
    pub fn get_lhs(&self) -> Vob {
        self.lhs.clone()
    }

    /// Replace `lhs` by the given `new_lhs`.
    #[inline]
    pub fn replace_lhs(&mut self, new_lhs: Vob) {
        self.lhs = new_lhs;
    }

    /// Add a `Vob` to `lhs`.
    ///
    /// Adding means xoring since we are adding a vector of bits.
    ///
    /// ex: [010011] + [011100] = [001111]
    #[inline]
    pub fn add_lhs(&mut self, added_lhs: &Vob) {
        self.lhs.xor(added_lhs);
    }

    /// Return an iterator over the positions of the set bits in `lhs`.
    /// ```text.
    /// lhs = [010111001] -> vec![1,3,4,5,8].iter()
    /// ```
    #[inline]
    pub fn iter_set_lhs(&self) -> IterSetBits<usize> {
        self.lhs.iter_set_bits(0..self.lhs.len())
    }

    /// Return a boolean indicating if the bit at position `var` is set in `lhs`.
    ///
    /// Will panic if var > lhs.len().
    pub fn is_var_set(&self, var: usize) -> bool {
        self.lhs
            .get(var)
            .expect("attempt to access a var outside of lhs")
    }

    /// Return an `Iterator` over `nodes`.
    #[inline]
    pub fn iter_nodes(&self) -> Iter<Id, Node> {
        self.nodes.iter()
    }

    /// Return an `Iterator` over `nodes`.
    #[inline]
    pub fn iter_mut_nodes(&mut self) -> IterMut<Id, Node> {
        self.nodes.iter_mut()
    }

    /// Get ref to the map of nodes
    #[inline]
    pub fn get_nodes(&self) -> &AHashMap<Id, Node> {
        &self.nodes
    }

    /// Get a mutable ref to the map of nodes
    #[inline]
    pub fn get_mut_nodes(&mut self) -> &mut AHashMap<Id, Node> {
        &mut self.nodes
    }

    /// Get the number of nodes of the level.
    #[inline]
    pub fn get_nodes_len(&self) -> usize {
        self.nodes.len()
    }

    /// Add a new `node` in the level with its `id` set at `n_id` and edges set to e0 and e1.
    pub fn add_edged_node(&mut self, n_id: Id, e0: Option<Id>, e1: Option<Id>) {
        let n = Node::with_edges(e0, e1);
        self.nodes.insert(n_id, n);
    }

    /// Add a new `node` in the level with its `id` set at `n_id` and edges set to `None`.
    pub fn add_new_node(&mut self, n_id: Id) {
        let n = Node::new();
        self.nodes.insert(n_id, n);
    }

    /// Replace `nodes` by the given `AHashMap` of nodes and resize it to reduce
    /// its memory footprint. We assume that no node will be insert after
    /// replacing the nodes hence the shrinking.
    pub fn replace_nodes(&mut self, nodes: AHashMap<Id, Node>) {
        self.nodes = nodes;
        self.nodes.shrink_to_fit();
    }

    /// Remove any node not present in parents and insert in parents the edges of the remaining nodes
    /// Return true if at least a node was removed
    pub fn remove_orphans(&mut self, parents: &mut AHashSet<Id>) -> bool {
        let len = self.nodes.len();
        let mut to_remove = AHashSet::with_capacity_and_hasher(len, Default::default());
        self.nodes.iter().for_each(|(id, node)| {
            if parents.remove(id) {
                if let Some(e0) = node.get_e0() {
                    parents.insert(e0);
                }
                if let Some(e1) = node.get_e1() {
                    parents.insert(e1);
                }
            } else {
                to_remove.insert(*id);
            }
        });
        self.remove_nodes_from_set(&to_remove);
        len > self.nodes.len()
    }

    /// Remove all nodes which ids are in the keys of the provided map
    pub fn remove_nodes_from_map(&mut self, map: &AHashMap<Id, Id>) {
        map.keys().for_each(|key| {
            self.nodes.remove(key);
        });
    }

    /// Remove all nodes which ids are in the provided set
    pub fn remove_nodes_from_set(&mut self, map: &AHashSet<Id>) {
        map.iter().for_each(|key| {
            self.nodes.remove(key);
        });
    }
    /// Remove a single node who has the provided Id
    pub fn remove_node(&mut self, to_remove: Id) {
        self.nodes.remove(&to_remove);
    }

    /// Check if `nodes` has at least one node with `e0` pointing to a valid `node` and one node
    /// with `e1` pointing to a valid `node`.
    ///
    /// Will return a tuple of boolean (`has_zero_edge`, `has_one_edge`), with a `true` indicating that
    /// at least one node has a valid edge.
    ///
    /// Short-circuited (will exit as soon as both type of edge has been found to avoid iterating the whole level).
    pub fn check_outgoing_edges(&self) -> (bool, bool) {
        let (mut has_zero_edge, mut has_one_edge) = (false, false);
        for node in self.nodes.iter() {
            if !has_zero_edge && node.1.get_e0().is_some() {
                has_zero_edge = true;
            }
            if !has_one_edge && node.1.get_e1().is_some() {
                has_one_edge = true;
            }
            // the first node must have an edge, it would be an orphan otherwise
            // so if has_zero_edge == has_one_edge then both == true
            if has_zero_edge == has_one_edge {
                break;
            }
        }
        (has_zero_edge, has_one_edge)
    }

    /// Flip the edges of all nodes in the level.
    pub fn flip_edges(&mut self) {
        self.nodes.iter_mut().for_each(|node| {
            node.1.flip_edges();
        });
    }

    /// Clear the nodes and return the first one
    /// We use this function when we need to absorb the source
    /// We can then simply grab the node, look at its edges and then
    /// delete the level
    pub fn pop_source(&mut self) -> Node {
        self.nodes
            .drain()
            .map(|(_, n)| n)
            .collect::<Vec<Node>>()
            .pop()
            .unwrap()
    }
}

impl fmt::Debug for Level {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "lhs {:?}", self.lhs)?;
        if self.nodes.is_empty() {
            write!(f, "No nodes at this level")?;
        } else {
            for n in self.nodes.iter() {
                writeln!(f, "{:?}", n)?;
            }
        }
        Ok(())
    }
}
