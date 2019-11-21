//! This is an implementation of a node inside a BDD (Binary Decision Diagram).
//!
//! A node is defined as 2 outgoing edges `e0` and `e1`.
//! An edge can either point to another node or to nothing (`None`).
//!
//! Because there is no formal link between node (like a pointer) an edge
//! can refer to a node id which no longer exist in the BDD if the node
//! was removed. Therefore it is necessary to clean the edges of the nodes
//! that can refer to a node that will be removed.

use crate::soc::Id;

/// A Node inside a Binary Decision Diagram
#[derive(Debug, Default)]
pub struct Node {
    e0: Option<Id>,
    e1: Option<Id>,
}

impl Node {
    /// Construct a new `Node` pointing to nothing.
    pub fn new() -> Node {
        Default::default()
    }
    /// Construct a new `Node` pointing to the specified edges.
    pub fn with_edges(e0: Option<Id>, e1: Option<Id>) -> Node {
        Node { e0, e1 }
    }

    /// Return a copy of the 0-edge
    #[inline]
    pub fn get_e0(&self) -> Option<Id> {
        self.e0
    }

    /// Return a copy of the 1-edge
    #[inline]
    pub fn get_e1(&self) -> Option<Id> {
        self.e1
    }

    /// Set `e0` the specified Id
    #[inline]
    pub fn connect_e0(&mut self, edge: Id) {
        self.e0 = Some(edge);
    }

    /// Set `e1` the specified Id
    #[inline]
    pub fn connect_e1(&mut self, edge: Id) {
        self.e1 = Some(edge);
    }

    /// Set `e0` to None.
    #[inline]
    pub fn disconnect_e0(&mut self) {
        self.e0 = None;
    }

    /// Set `e1` to None.
    #[inline]
    pub fn disconnect_e1(&mut self) {
        self.e1 = None;
    }

    /// Point `e0` to `e1` and `e1` to `e0`, flipping the edges.
    #[inline]
    pub fn flip_edges(&mut self) {
        let e1 = self.e0;
        self.e0 = self.e1;
        self.e1 = e1;
    }
}
