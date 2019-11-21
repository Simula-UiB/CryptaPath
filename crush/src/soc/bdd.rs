//! This is implementation of a `BDD` (Binary Decision Diagram).
//! A `BDD` is defined by an `id` and an array of `levels`.
//! For performance purpose we also store the `next_id` variable
//! which is the id to use when inserting a new node inside the BDD.
//! This ensure that we never have 2 nodes with the same id inside.
//!
//! /!\ Because we are joining BDDs and this involve merging the sets
//! of ids of the nodes we have to make sure that there is no similar
//! id between 2 BDD. The way we do it is by reducing the sets of possible
//! id and making the nodes id dependant on the Id of the BDD in which
//! they are created.
//! All nodes Id are equal to `next_id * 10000 + bdd_id`.
//! This assumes that:
//! - You have less than 10 000 BDDs in your system
//! - Your bdd_id is between 0 and 10 000
//! - You will create less than ~2**53 nodes in your BDD (would overflow a 64 bits usize otherwise)
//! - Generally you are running on a 64 bit system
//!
//! Out of the array of `levels` 2 are specific : the first and the last.
//! While they are stored as any level, the first level will only be one node
//! and the last will also be one node with both outgoing edges set to None
//! and the all zero vector as its lhs.
//! This is important to remember for the functions that are :
//! - collecting the lhs (avoid the last one)
//! - removing the dead end nodes (skip the last level)
//! - removing the orphan nodes (skip the first level)

use crate::soc::node::Node;
use crate::soc::{level::Level, Id};
use crate::{AHashMap, AHashSet};
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::hash::BuildHasherDefault;
use vob::Vob;

/// A `LinEq` is a linear equation found in the BDD.
/// A level which has only outgoing 1edges or 0edges
/// can be absorbed and its equation and value extracted as a `LinEq`.
/// Those will be put inside a `lin_bank` (system level) and
/// be used for solving the system at the end

#[derive(Default, Debug, Clone)]
pub struct LinEq {
    lhs: Vob,
    rhs: bool,
}

impl LinEq {
    /// Construct a new `LinEq` with provided parameters
    #[inline]
    pub fn new(lhs: Vob, rhs: bool) -> LinEq {
        LinEq { lhs, rhs }
    }

    /// Return a copy of the `lhs` of the `LinEq`
    #[inline]
    pub fn get_lhs(&self) -> Vob {
        self.lhs.clone()
    }

    /// Return a copy of the `rhs` of the `LinEq`
    #[inline]
    pub fn get_rhs(&self) -> bool {
        self.rhs
    }

    /// Return an `Option` around the position of the max
    /// set bit of the `lhs` of the `LinEq`
    ///
    /// `None` is returned if lhs -> all zero vector
    #[inline]
    pub fn get_lhs_max_set_bit(&self) -> Option<usize> {
        self.lhs.iter_set_bits(..).last()
    }

    /// Add a `LinEq` to the current LinEq with
    /// adding meaning xoring `lhs` and `rhs`
    #[inline]
    pub fn add_lin_eq(&mut self, lin_eq: &LinEq) {
        self.lhs.xor(&lin_eq.get_lhs());
        self.rhs ^= lin_eq.get_rhs();
    }
}

/// A Binary Decision Diagram (see module documentation for more details)
#[derive(Default)]
pub struct Bdd {
    levels: Vec<Level>,
    id: Id,
    next_id: usize,
}

impl Bdd {
    /// Construct a new `Bdd` with default parameters
    pub fn new() -> Bdd {
        Default::default()
    }

    /// Set the id of the `Bdd` to the given id
    #[inline]
    pub fn set_id(&mut self, id: Id) {
        self.id = id;
    }

    /// Return the id of the `Bdd`
    #[inline]
    pub fn get_id(&self) -> Id {
        self.id
    }

    /// Set next id for the next node to be inserted
    #[inline]
    pub fn set_next_id(&mut self, next_id: usize) {
        self.next_id = next_id;
    }

    /// Return the index of the last level
    #[inline]
    pub fn get_sink_level_index(&self) -> usize {
        self.levels.len() - 1
    }

    /// Add an empty level at the end of the `Bdd`
    #[inline]
    pub fn add_level(&mut self) {
        self.levels.push(Level::new());
    }

    /// Push at the end of the `Bdd` an existing level (used for joining BDDs)
    #[inline]
    pub fn add_existing_level(&mut self, level: Level) {
        self.levels.push(level);
    }

    /// Return an iterator over the levels of the `Bdd`
    #[inline]
    pub fn iter_levels(&self) -> std::slice::Iter<Level> {
        self.levels.iter()
    }

    /// Return a draining iterator over the levels of the `Bdd`
    #[inline]
    pub fn drain_levels(&mut self) -> std::vec::Drain<Level> {
        self.levels.drain(..)
    }

    /// Return the number of levels of the `Bdd`
    #[inline]
    pub fn get_levels_size(&self) -> usize {
        self.levels.len()
    }

    /// Return the number of variable in the lhs of the BDD
    #[inline]
    pub fn get_nvar_size(&self) -> usize {
        self.levels[0].get_lhs().len()
    }

    /// Return a vector containing all the left hand side of each level of the BDD
    ///
    /// The last level is skipped as it would be an all zero vector (the sink doesn't have a left hand side)
    pub fn get_lhs(&self) -> Vec<Vob> {
        self.levels
            .iter()
            .take(self.levels.len() - 1)
            .map(|level| level.get_lhs())
            .collect()
    }

    /// Return the total number of nodes inside the BDD
    pub fn get_size(&self) -> usize {
        self.levels
            .iter()
            .fold(0, |acc, level| acc + level.get_nodes_len())
    }

    /// Call the `set_lhs` function on the level specified by `level_index` with the given parameters
    /// See the Level documentation for more information
    pub fn set_lhs_level(&mut self, level_index: usize, vars: Vec<usize>, var_len: usize) {
        self.levels[level_index].set_lhs(vars, var_len);
    }

    /// Repeatedly calls the `add_node` function on the level specified by the `level_index`
    /// for each id in `nodes_id`
    /// /!\ no update is made to self.next_id, you are expected to set it yourself
    pub fn add_nodes_to_level(&mut self, level_index: usize, nodes_id: Vec<Id>) {
        let mut nodes = Vec::new();
        for node_id in nodes_id.iter() {
            let new_id = Id::new(**node_id * 10000 + *self.id);
            self.levels[level_index].add_new_node(new_id);
            nodes.push(new_id);
        }
    }

    /// Iterate through the levels of the BDD to find a node of id `parent` and set its
    /// edge 0 or 1 (depending on the value of `edge`) to the provided `child_id`
    ///
    /// This is obviously very slow on large BDD but this method is only use when constructing
    /// the BDDs initially making it virtually no cost as BDDs are usually extremely small
    /// at this stage
    pub fn connect_nodes_from_spec(&mut self, parent: Id, child_id: Id, edge: i8) {
        assert!(edge == 0 || edge == 1);
        let child_id = Id::new(*child_id * 10000 + *self.id);
        let parent_id = Id::new(*parent * 10000 + *self.id);
        self.levels.iter_mut().for_each(|level| {
            if let Some(n) = level.get_mut_nodes().get_mut(&parent_id) {
                match edge {
                    0 => n.connect_e0(child_id),
                    1 => n.connect_e1(child_id),
                    _ => panic!("impossible due to the assert"),
                }
                return;
            }
        });
    }

    /// Remove every node which represent a dead-end starting from the level start going upwards
    /// and "clean" the BDD by removing any reference to a removed node.
    ///
    /// A dead-end is defined as a `node` which has both `e0` and `e1` pointing to `None`
    /// Therefore the sink of the BDD is considered a dead-end and would be removed if start = last level.
    /// Always make sure that, it is not the case to avoid deleting the whole bdd.
    ///
    /// We start by checking that all edges are "valid", meaning that there are not refering to an
    /// Id absent of the level below (this would indicate that the node has been deleted). If the id
    /// has no corresponding node we set the edge to `None`. If both edges are set to `None` the node will
    /// be removed.
    ///
    /// Short circuited -> will exit when no dead end was found in the previous level
    fn remove_all_dead_ends_start(&mut self, start: usize) {
        for i in (0..=start).rev() {
            let mut to_remove: AHashSet<Id> = AHashSet::with_capacity_and_hasher(
                self.levels[i].get_nodes_len(),
                Default::default(),
            );
            let (above, below) = self.levels.split_at_mut(i + 1);
            above
                .last_mut()
                .unwrap()
                .iter_mut_nodes()
                .for_each(|(id, node)| {
                    let mut edges = (false, false);
                    if let Some(e0) = node.get_e0() {
                        match below[0].get_nodes().get(&e0) {
                            Some(_) => edges.0 = true,
                            None => node.disconnect_e0(),
                        }
                    }
                    if let Some(e1) = node.get_e1() {
                        match below[0].get_nodes().get(&e1) {
                            Some(_) => edges.1 = true,
                            None => node.disconnect_e1(),
                        }
                    }
                    if !edges.0 && !edges.1 {
                        to_remove.insert(*id);
                    }
                });
            if to_remove.is_empty() {
                return;
            }
            self.levels[i].remove_nodes_from_set(&to_remove);
            to_remove.clear()
        }
    }

    /// Remove every orphan node starting from the level `start` going downwards
    ///
    /// An orphan is defined as a node which doesn't have any node pointing to him in the levels above
    ///
    /// To keep track of the expected child we start from the `level` `start-1` and keep all the edges inside a hashset.
    /// We then iterate through every level, removing any `node` which is not in the set and adding the remaining outgoing edges to the set.
    /// `start` should be the level where you want the removing to begin and therefore never equal to `0`
    /// Short circuited -> will exit when no orphans was found in the previous level
    fn remove_orphans_start(&mut self, start: usize) {
        assert!(start != 0);
        let mut parents: AHashSet<Id> = AHashSet::with_capacity_and_hasher(
            self.levels[start - 1].get_nodes_len(),
            Default::default(),
        );
        self.levels[start - 1].iter_nodes().for_each(|(_, node)| {
            if let Some(e0) = node.get_e0() {
                parents.insert(e0);
            }
            if let Some(e1) = node.get_e1() {
                parents.insert(e1);
            }
        });
        for i in start..self.levels.len() - 1 {
            let removed = self.levels[i].remove_orphans(&mut parents);
            if !removed {
                return;
            }
        }
    }

    /// Perform the swap operation on `level_1` and `level_2`.
    /// `level_1` should be just above `level_2`
    ///
    /// The swapping algorithm is as follow :
    /// The generated_nodes vector will replace the `level_2` when the algorithm is over
    ///
    /// For each node at `level_1` :
    ///
    /// check if from the `node` the path 0-0 or the path 1-0 exists :
    /// if it doesn't exists, disconnect the 0edge node;
    /// if it exists create `new_node`, connect `node` to `new_node` along the 0edge,
    /// connect `new_node` to the corresponding edges of the old child of `node`
    /// (from the node at `level_1` the path 0-0 stay 0-0 and 1-0 become 0-1)
    /// push `new_node` to generated_nodes
    ///
    ///
    /// check if from the node the path 0-1 or the path 1-1 exists :
    /// if it doesn't exists, disconnect the 1edge node;
    /// if it exists create `new_node`, connect `node` to `new_node` along the 1edge,
    /// connect `new_node` to the corresponding edges of the old child of `node`
    /// (therefore from the node at `level_1` the path 0-1 become 1-0 and 1-1 stay 1-1),
    /// push `new_node` to `generated_nodes`.
    ///
    /// To avoid creating nodes representing the same function (same edges as an already existing node)
    /// a hashmap is used to keep track of the already existing function: `known_functions`.
    /// If the function represented by the node we want to generate already exists
    /// -> instead of generating, connect `node` to this already existing node
    ///
    /// Finally swap the lhs of `level_1` and `level_2`
    pub fn swap(&mut self, level_index_above: usize, level_index_below: usize) {
        assert!(level_index_above + 1 == level_index_below);
        let max_level_size = self.levels[level_index_below].get_nodes_len() * 2;
        let mut known_functions: AHashMap<(Option<Id>, Option<Id>), Id> =
            AHashMap::with_capacity_and_hasher(max_level_size, Default::default());
        let mut nodes: AHashMap<Id, Node> =
            AHashMap::with_capacity_and_hasher(max_level_size, Default::default());
        let (above, below) = self.levels.split_at_mut(level_index_above + 1);
        let mut next_id = self.next_id;
        let bdd_id = *self.id;
        above
            .last_mut()
            .unwrap()
            .iter_mut_nodes()
            .for_each(|(_, node)| {
                let e0_edges = match node.get_e0() {
                    Some(e0) => match below[0].get_nodes().get(&e0) {
                        Some(e0_below) => (e0_below.get_e0(), e0_below.get_e1()),
                        None => {
                            node.disconnect_e0();
                            (None, None)
                        }
                    },
                    None => (None, None),
                };
                let e1_edges = match node.get_e1() {
                    Some(e1) => match below[0].get_nodes().get(&e1) {
                        Some(e1_below) => (e1_below.get_e0(), e1_below.get_e1()),
                        None => {
                            node.disconnect_e1();
                            (None, None)
                        }
                    },
                    None => (None, None),
                };
                if e0_edges.0.is_some() || e1_edges.0.is_some() {
                    match known_functions.get(&(e0_edges.0, e1_edges.0)) {
                        Some(existing_node) => {
                            node.connect_e0(*existing_node);
                        }
                        None => {
                            let new_id = {
                                next_id += 1;
                                Id::new(next_id * 10000 + bdd_id)
                            };
                            node.connect_e0(new_id);
                            nodes.insert(new_id, Node::with_edges(e0_edges.0, e1_edges.0));
                            known_functions.insert((e0_edges.0, e1_edges.0), new_id);
                        }
                    }
                } else {
                    node.disconnect_e0()
                }
                if e0_edges.1.is_some() || e1_edges.1.is_some() {
                    match known_functions.get(&(e0_edges.1, e1_edges.1)) {
                        Some(existing_node) => {
                            node.connect_e1(*existing_node);
                        }
                        None => {
                            let new_id = {
                                next_id += 1;
                                Id::new(next_id * 10000 + bdd_id)
                            };
                            node.connect_e1(new_id);
                            nodes.insert(new_id, Node::with_edges(e0_edges.1, e1_edges.1));
                            known_functions.insert((e0_edges.1, e1_edges.1), new_id);
                        }
                    }
                } else {
                    node.disconnect_e1()
                }
            });
        self.next_id = next_id;
        self.levels[level_index_below].replace_nodes(nodes);
        let lhs_1 = self.levels[level_index_above].get_lhs();
        let lhs_2 = self.levels[level_index_below].get_lhs();
        self.levels[level_index_above].replace_lhs(lhs_2);
        self.levels[level_index_below].replace_lhs(lhs_1);
    }

    /// Perform the add operation between `level_1` and `level_2`
    ///
    /// `level_1` should be < to `level_2`
    ///
    /// If `level_1` is not directly above `level_2`, perform swapping until it is
    ///
    /// The adding algorithm is as follow :
    /// the `generated_nodes` vector will replace the `level_2` when the algorithm is over
    ///
    /// For each `node` at `level_1` :
    ///
    /// if the `node` as a 1edge, create `new_node`, connect `node` to `new_node` along the 1edge,
    /// connect `new_node` to the flipped edges of the old 1edge child of `node`
    /// (therefore from the node at `level_1` the path 1-0 become 1-1 and 1-1 become 1-0),
    /// push `new_node` to `generated_nodes`
    ///
    /// To avoid creating nodes representing the same function (same edges as an already existing node)
    /// a hashmap is used to keep track of the already existing function:`known_functions`.
    /// If the function represented by the node we want to generate already exists
    /// -> instead of generating, connect node to this already existing node
    ///
    /// For the 0edge we just perform the check that it is not representing an already existing
    /// function, if no we add it to the `generated_nodes` and `known_function` else connect
    /// to the already existing node
    ///
    /// Finally add the `lhs` of `level_1` to `level_2`
    pub fn add(&mut self, mut level_index_above: usize, level_index_below: usize) {
        assert!(level_index_above < level_index_below);
        while level_index_below > level_index_above + 1 {
            self.swap(level_index_above, level_index_above + 1);
            level_index_above += 1;
        }
        let max_level_size = self.levels[level_index_below].get_nodes_len() * 2;
        let mut nodes: AHashMap<Id, Node> =
            AHashMap::with_capacity_and_hasher(max_level_size, Default::default());
        let mut known_functions: AHashMap<(Option<Id>, Option<Id>), Id> =
            AHashMap::with_capacity_and_hasher(max_level_size, Default::default());
        let (above, below) = self.levels.split_at_mut(level_index_above + 1);
        let mut next_id = self.next_id;
        let bdd_id = *self.id;
        for (_, node) in above.last_mut().unwrap().iter_mut_nodes() {
            if let Some(e0) = node.get_e0() {
                match below[0].get_nodes().get(&e0) {
                    Some(e0_node) => {
                        let e0_edges = (e0_node.get_e0(), e0_node.get_e1());
                        match known_functions.get(&e0_edges) {
                            Some(existing_node) => {
                                node.connect_e0(*existing_node);
                            }
                            None => {
                                nodes.insert(e0, Node::with_edges(e0_edges.0, e0_edges.1));
                                known_functions.insert(e0_edges, e0);
                            }
                        }
                    }
                    None => node.disconnect_e0(),
                }
            };
            if let Some(e1) = node.get_e1() {
                match below[0].get_nodes().get(&e1) {
                    Some(e1_node) => {
                        let e1_edges = (e1_node.get_e1(), e1_node.get_e0());
                        match known_functions.get(&(e1_edges)) {
                            Some(existing_node) => {
                                node.connect_e1(*existing_node);
                            }
                            None => {
                                let new_id = {
                                    next_id += 1;
                                    Id::new(next_id * 10000 + bdd_id)
                                };
                                node.connect_e1(new_id);
                                nodes.insert(new_id, Node::with_edges(e1_edges.0, e1_edges.1));
                                known_functions.insert(e1_edges, new_id);
                            }
                        }
                    }
                    None => node.disconnect_e1(),
                }
            }
        }
        self.next_id = next_id;
        self.levels[level_index_below].replace_nodes(nodes);
        let lhs_1 = self.levels[level_index_above].get_lhs();
        self.levels[level_index_below].add_lhs(&lhs_1);
    }

    /// Perform a "drop" of a `level` -> assume that the `level` contains an independent variable,
    /// `swap` the `level` to the bottom just above the sink,
    /// Connect each edge of the `level` above to the sink if they were connected to the `level` to drop,
    /// remove the level to drop,
    /// finally merge the equal nodes in the bdd.
    pub fn drop(&mut self, mut level_index: usize) {
        while level_index != self.get_levels_size() - 2 {
            self.swap(level_index, level_index + 1);
            level_index += 1;
        }
        let len = self.get_levels_size() - 1;
        let (above, sink) = self.levels.split_at_mut(len);
        if level_index != 0 {
            if let Some((sink_id, _)) = sink[0].iter_nodes().next() {
                above[len - 2].iter_mut_nodes().for_each(|(_, node)| {
                    if node.get_e0().is_some() {
                        node.connect_e0(*sink_id);
                    }
                    if node.get_e1().is_some() {
                        node.connect_e1(*sink_id);
                    }
                });
            }
        }
        self.levels.remove(level_index);
        if level_index > 1 {
            self.merge_equals_node_start(level_index - 1);
        }
    }

    /// Perform an "absorbtion" of a `level` -> assume the lhs is equal to `edge`,
    /// connect each parent of the nodes located at `level_index` to its child 0/1edge (depending of the valeur of `edge`).
    /// The opposite edges are now non-valid (if the lhs is equal to zero, cannot be equal to one and viceversa).
    /// The level is then remove and reducing is perform on the bdd (removing orphans and dead ends).
    pub fn absorb(&mut self, level_index: usize, edge: bool) {
        let mut new_level = AHashMap::with_capacity_and_hasher(
            self.levels[level_index].get_nodes_len(),
            Default::default(),
        );

        // If the level to absorb is the source of the bdd, different strategy
        if level_index == 0 {
            self.absorb_source(edge);
            return;
        }

        if !edge {
            for (id, node) in self.levels[level_index].iter_nodes() {
                let e0 = node.get_e0();
                if let Some(e0) = e0 {
                    new_level.insert(*id, e0);
                }
            }
        } else {
            for (id, node) in self.levels[level_index].iter_nodes() {
                let e1 = node.get_e1();
                if let Some(e1) = e1 {
                    new_level.insert(*id, e1);
                }
            }
        }
        // If there is no valid outgoing edges then there is no solution
        // Basically this means that we absorbed a level along an edge and
        // the level had only outgoing edges of the other type.
        // This would be a 0 = 1
        if new_level.is_empty() {
            panic!("System has no solutions")
        }
        self.point_all_parents_to_new_level_map(&new_level, level_index - 1, level_index);
        self.levels.remove(level_index);
        self.remove_all_dead_ends_start(level_index - 1);
        self.remove_orphans_start(level_index);
        self.merge_equals_node_start(level_index - 1);
    }

    /// Absorb the source of the bdd along the edge precised.
    /// To absorb it we remove the opposing edge of the next level.
    /// The level 0 is then removed and then the orphans removed starting at new level 1
    fn absorb_source(&mut self, edge: bool) {
        let node = &self.levels[0].pop_source();
        // if the top node has both edges pointing to same node, we don't need to remove the wrong edge
        if node.get_e0() != node.get_e1() {
            if !edge {
                if let Some(e1) = node.get_e1() {
                    self.levels[1].remove_node(e1);
                }
            } else if let Some(e0) = node.get_e0() {
                self.levels[1].remove_node(e0);
            }
        }
        self.levels.remove(0);
        // If there is not valid outgoing edge then there is no solution
        if self.levels[0].get_nodes_len() == 0 {
            panic!("System has no solutions")
        }
        self.remove_orphans_start(1);
    }

    /// Iterate through the bdd to find linear equations
    /// A linear equation is found when a level has only outgoing 0edges
    /// or outoing 1edges
    /// The equation is then extracted as a LinEq and the level absorbed
    /// Loop until no equation are left to absorb
    pub fn scan_absorb_lin_eq(&mut self) -> Vec<LinEq> {
        let mut lin_eqs_absorbed = Vec::new();
        loop {
            let mut absorbed = false;
            // We skip the last (which has no outgoing edges at all)
            for (i, level) in self.levels.iter().take(self.levels.len() - 1).enumerate() {
                // in the unlikely event that there is a 0 level remaining in the BDD
                // we absorb it but the equation is 0 = 0 so we don't grab it
                if level.iter_set_lhs().count() == 0 {
                    self.absorb(i, false);
                    absorbed = true;
                    break;
                }
                let (has_0edge, has_1edge) = level.check_outgoing_edges();
                if !has_0edge {
                    let lin_eq = LinEq::new(level.get_lhs(), true);
                    lin_eqs_absorbed.push(lin_eq);
                    self.absorb(i, true);
                    absorbed = true;
                    break;
                } else if !has_1edge {
                    let lin_eq = LinEq::new(level.get_lhs(), false);
                    lin_eqs_absorbed.push(lin_eq);
                    self.absorb(i, false);
                    absorbed = true;
                    break;
                }
            }
            if !absorbed {
                break;
            }
        }
        lin_eqs_absorbed
    }

    /// Used to remove any jumping edges in a bdd, ensuring that if a node has a parent
    /// it is located in the level just above. This is important for performance since we don't
    /// keep track of the parents of a node.
    ///
    /// Should be use only when loading the system at the start (jumping edges cannot appear after).
    pub fn add_same_edges_node_at_level(&mut self, level_index: usize) {
        let mut changed = false;
        if level_index != 0 {
            let mut childs: HashSet<Id, BuildHasherDefault<ahash::AHasher>> =
                AHashSet::with_capacity_and_hasher(
                    self.levels[level_index - 1].get_nodes_len(),
                    Default::default(),
                );
            for (_, node) in self.levels[level_index - 1].iter_nodes() {
                if let Some(e0) = node.get_e0() {
                    childs.insert(e0);
                }
                if let Some(e1) = node.get_e1() {
                    childs.insert(e1);
                }
            }
            for (id, _) in self.levels[level_index].iter_nodes() {
                childs.remove(id);
            }
            let mut new_level: HashMap<Id, Id, BuildHasherDefault<ahash::AHasher>> =
                AHashMap::with_capacity_and_hasher(childs.len(), Default::default());
            if !childs.is_empty() {
                changed = true;
            }
            for node in childs.iter() {
                let new_id = {
                    let next_id = self.next_id + 1;
                    self.next_id = next_id;
                    Id::new(next_id * 10000 + *self.id)
                };
                self.levels[level_index].add_edged_node(new_id, Some(*node), Some(*node));
                new_level.insert(*node, new_id);
            }
            if changed {
                self.point_all_parents_to_new_level_map(&new_level, 0, level_index);
            }
        }
    }

    /// Merge nodes which represent the same function in a level.
    /// Start with the level_index and goes upwards.
    ///
    /// Short circuited -> will stop when no change were found in the previous level.
    pub fn merge_equals_node_start(&mut self, mut level_index: usize) {
        let mut changed = true;
        let max_size_map = self.levels[level_index].get_nodes_len();
        let mut known_functions: HashMap<
            (Option<Id>, Option<Id>),
            Id,
            BuildHasherDefault<ahash::AHasher>,
        > = AHashMap::with_capacity_and_hasher(max_size_map, Default::default());
        let mut map: HashMap<Id, Id, BuildHasherDefault<ahash::AHasher>> =
            AHashMap::with_capacity_and_hasher(max_size_map, Default::default());
        while changed && level_index > 1 {
            changed = false;
            for (id, node) in self.levels[level_index].iter_nodes() {
                match known_functions.get(&(node.get_e0(), node.get_e1())) {
                    Some(existing_node) => {
                        changed = true;
                        map.insert(*id, *existing_node);
                    }
                    None => {
                        known_functions.insert((node.get_e0(), node.get_e1()), *id);
                    }
                };
            }
            self.point_all_parents_to_new_level_map(&map, level_index - 1, level_index);
            self.levels[level_index].remove_nodes_from_map(&map);
            known_functions.clear();
            map.clear();
            level_index -= 1;
        }
    }

    /// For all `nodes` located on the range `level_start..level_max` (level_max not included) :
    ///
    /// point their existing edges to a new node following the `HashMap` passed as a parameter.
    /// The `HashMap` should contain the `Id` of the old node as the key and the id of the new node as its content
    fn point_all_parents_to_new_level_map(
        &mut self,
        map: &AHashMap<Id, Id>,
        level_start: usize,
        level_max: usize,
    ) {
        self.levels
            .iter_mut()
            .skip(level_start)
            .take(level_max - level_start)
            .for_each(|level| {
                level.iter_mut_nodes().for_each(|(_, node)| {
                    let e0 = node.get_e0();
                    if let Some(e0) = e0 {
                        if let Some(new_node) = map.get(&e0) {
                            node.connect_e0(*new_node);
                        }
                    }
                    let e1 = node.get_e1();
                    if let Some(e1) = e1 {
                        if let Some(new_node) = map.get(&e1) {
                            node.connect_e1(*new_node);
                        }
                    }
                });
            });
    }

    /// Use when joining BDDs to merge the source of the BDD join to below
    /// with the sink of the BDD above it
    pub fn merge_sink_source(&mut self, sink_level_index: usize) {
        let (sink_bdd, source_bdd) = self.levels.split_at_mut(sink_level_index + 1);
        if let Some((_, source)) = source_bdd[0].iter_nodes().next() {
            if let Some((_, sink)) = sink_bdd.last_mut().unwrap().iter_mut_nodes().next() {
                if let Some(e0) = source.get_e0() {
                    sink.connect_e0(e0);
                }
                if let Some(e1) = source.get_e1() {
                    sink.connect_e1(e1);
                }
            }
            let source_lhs = self.levels[sink_level_index + 1].get_lhs();
            self.levels[sink_level_index].replace_lhs(source_lhs);
        }
        self.levels.remove(sink_level_index + 1);
    }

    /// Returns a `Vec` of all valid paths of a `Bdd`.
    ///
    /// A path is defined as a `Vec` of `LinEq` made of the `lhs` of the `levels`
    /// and an outgoing edge of the `level`.
    ///
    /// /!\ This is VERY SLOW you should avoid using it on a big BDD
    ///
    /// To produce all path we start from the top to the bottom.
    /// We keep a stack (LIFO) of tuples containing the state of the path,
    /// the index of the `level` and the reference to the `node`. We push to
    /// everytime we find a `node` that has both edges not set to `None`.
    /// When we reach the sink we go back to the stack to find the next path
    /// up until the stack is exhausted.
    /// If a BDD contain more that 20 paths we only return the first 20 to avoid
    /// exploding in memory size.
    pub fn get_all_valid_path(&self) -> Vec<Vec<LinEq>> {
        if self.get_sink_level_index() == 0 {
            return vec![vec![]];
        }
        let mut paths = Vec::new();
        let mut last_double_edge_node: Vec<(Vec<LinEq>, usize, (Option<Id>, Option<Id>))> =
            Vec::new();
        while !last_double_edge_node.is_empty() || paths.is_empty() {
            let mut path;
            let mut node: (Option<Id>, Option<Id>);
            let mut level_index;
            let mut visited = if !last_double_edge_node.is_empty() {
                // We have something to go back found in a previous path
                let last = last_double_edge_node.pop().unwrap();
                path = last.0;
                level_index = last.1;
                node = last.2;
                true
            } else {
                // we are starting from top

                if let Some((_, n)) = self.levels[0].iter_nodes().next() {
                    node = (n.get_e0(), n.get_e1())
                } else {
                    panic!("Cannot happen")
                }
                path = Vec::new();
                level_index = 0;
                false
            };
            //while we haven't reach the sink
            loop {
                // sink reached
                if node.0.is_none() && node.1.is_none() {
                    break;
                }
                // Already been there so we already know the e0 path -> follow e1
                if visited {
                    path.push(LinEq::new(self.levels[level_index].get_lhs(), true));
                    let next = node.1.unwrap();
                    if let Some(n) = self.levels[level_index + 1].get_nodes().get(&next) {
                        node = (n.get_e0(), n.get_e1());
                    }
                    level_index += 1;
                    visited = false;
                    continue;
                }
                // double edge node -> let's store it for later
                if node.0.is_some() && node.1.is_some() {
                    last_double_edge_node.push((path.clone(), level_index, node));
                }
                let has_e0 = node.0;
                if let Some(e0) = has_e0 {
                    path.push(LinEq::new(self.levels[level_index].get_lhs(), false));
                    if let Some(n) = self.levels[level_index + 1].get_nodes().get(&e0) {
                        node = (n.get_e0(), n.get_e1());
                    }
                    level_index += 1;
                    continue;
                }
                let has_e1 = node.1;
                if let Some(e1) = has_e1 {
                    path.push(LinEq::new(self.levels[level_index].get_lhs(), true));
                    if let Some(n) = self.levels[level_index + 1].get_nodes().get(&e1) {
                        node = (n.get_e0(), n.get_e1());
                    }
                    level_index += 1;
                    continue;
                }
            }
            paths.push(path);
            //just checking to avoid exploding in memory if called on a really large bdd
            if paths.len() > 20 {
                return paths;
            }
        }
        paths
    }

    /// Count the number of paths inside a `Bdd`.
    ///
    /// To count the number of paths we go from bottom to top
    ///
    /// We keep a map of the previous level containing a mapping from `Id` to `weight`.
    /// `weight` is the number of path that leads to that node and is the sum of the weights
    /// of the childs of the node. If the weight of a child is zero the child is the sink
    /// therefore 0 -> 1 path.
    ///
    /// When we reach the top the `previous_level_weight` will contain only the top node
    /// So we can grab it and its weight will be the number of paths of the bdd
    /// If the bdd is only a sink (number of level < 2), we return `0`
    pub fn count_paths(&self) -> u128 {
        let mut previous_level_weigths: HashMap<Id, u128, BuildHasherDefault<ahash::AHasher>> =
            AHashMap::with_hasher(Default::default());
        if self.levels.len() < 2 {
            return 0;
        }
        for level in self.iter_levels().rev() {
            let mut current_level_weigths = AHashMap::with_hasher(Default::default());
            for (id, node) in level.iter_nodes() {
                let weight = match node.get_e0() {
                    Some(e0_id) => match previous_level_weigths.get(&e0_id) {
                        Some(weight) => {
                            if *weight == 0 {
                                1
                            } else {
                                *weight
                            }
                        }
                        None => 0,
                    },
                    None => 0,
                };
                let (weight, overflow) = weight.overflowing_add(match node.get_e1() {
                    Some(e1_id) => match previous_level_weigths.get(&e1_id) {
                        Some(weight) => {
                            if *weight == 0 {
                                1
                            } else {
                                *weight
                            }
                        }
                        None => 0,
                    },
                    None => 0,
                });
                if overflow {
                    panic!("the bdd has over 2^128 paths")
                }
                current_level_weigths.insert(*id, weight);
            }
            previous_level_weigths = current_level_weigths;
        }
        *previous_level_weigths.iter().next().unwrap().1
    }

    /// Replace a variable in all the lhs of the bdd by a linear combination.
    /// If the linear combination is equal to true:flip all the edges of the level.
    /// If when replacing the lhs a zero level is created -> absorb it along its zero edges.
    pub fn replace_var_in_bdd(&mut self, var: usize, eq: &LinEq) {
        let mut to_absorbe: Vec<usize> = Vec::with_capacity(self.levels.len());
        // We should be skipping the last level, but since we are explicitly checking that
        // the level has the var bit set and the last level has an all-zero lhs
        // it won't be affected and it's easier to let it go instead of changing the iterator
        self.levels.iter_mut().enumerate().for_each(|(i, level)| {
            if level.is_var_set(var) {
                level.add_lhs(&eq.get_lhs());
                if eq.get_rhs() {
                    level.flip_edges();
                }
                if level.iter_set_lhs().next() == None {
                    //No bits are set -> zero level
                    to_absorbe.push(i);
                }
            }
        });
        for _ in 0..to_absorbe.len() {
            self.absorb(to_absorbe.pop().unwrap(), false);
        }
    }
}

impl fmt::Debug for Bdd {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "Bdd id {}", *self.id)?;
        if self.levels.is_empty() {
            writeln!(f, "No levels on this BDD")?;
        }
        for (i, level) in self.levels.iter().enumerate() {
            writeln!(f, "level {}", i)?;
            writeln!(f, "{:?}", level)?;
        }
        Ok(())
    }
}

/// Mainly implemented for testing
///
/// Both `Bdd` should be reduced before comparing.
///
/// Since we cannot rely on the order or the id of the nodes,
/// we compare 2 bdd by mapping nodes to each other following their edges.
/// Nodes should always represent the same function as their mapped node
/// on the other bdd or else the bdds are not equal.
impl PartialEq for Bdd {
    fn eq(&self, other: &Bdd) -> bool {
        if self.get_levels_size() != other.get_levels_size() {
            return false;
        }
        if self.get_size() != other.get_size() {
            return false;
        }
        if self.get_lhs() != other.get_lhs() {
            return false;
        }
        // node_mapping will map the id of a node in self to ref of a node in other
        let mut node_mapping: HashMap<Id, Id, BuildHasherDefault<ahash::AHasher>> =
            AHashMap::with_hasher(Default::default());

        // Initialize the hashmap with the sources
        node_mapping.insert(
            *self
                .iter_levels()
                .next()
                .unwrap()
                .iter_nodes()
                .next()
                .unwrap()
                .0,
            *other
                .iter_levels()
                .next()
                .unwrap()
                .iter_nodes()
                .next()
                .unwrap()
                .0,
        );
        // There has to be a way cleaner to do that
        // but again this was mainly implemented for testing
        // so we didn't bother making it optimized
        for (level_index, level_self) in self.iter_levels().enumerate() {
            for (id_self, node_self) in level_self.iter_nodes() {
                let (e0_self, e1_self) = (node_self.get_e0(), node_self.get_e1());
                let id_other = node_mapping.get(id_self).unwrap();
                if let Some(node_other) = other.levels[level_index].get_nodes().get(&id_other) {
                    let (e0_other, e1_other) = (node_other.get_e0(), node_other.get_e1());
                    match e0_self {
                        Some(e0_self) => match e0_other {
                            Some(e0_other) => {
                                node_mapping.insert(e0_self, e0_other);
                            }
                            None => {
                                return false;
                            }
                        },
                        None => {
                            if e0_other.is_some() {
                                return false;
                            }
                        }
                    }
                    match e1_self {
                        Some(e1_self) => match e1_other {
                            Some(e1_other) => {
                                node_mapping.insert(e1_self, e1_other);
                            }
                            None => {
                                return false;
                            }
                        },
                        None => {
                            if e1_other.is_some() {
                                return false;
                            }
                        }
                    }
                }
            }
        }
        true
    }
}

impl Eq for Bdd {}
