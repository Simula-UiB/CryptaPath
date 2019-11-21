//! Module providing the structures to represent a system of equation represented by a set of
//! binary decision diagram (Bdd) and exposing the apis to absorb all the linear dependencies
//! inside to solve it.

pub mod bdd;
mod level;
mod node;
pub mod system;
pub mod utils;

use std::fmt::{self, Display};
use std::ops::Deref;

#[macro_export]
/// Macro to generate bdds :
///
/// ### Example :
///
/// ```text.
/// let bdd = bdd!(5;0;[("1+2",[(1;2,3)]);("3+2",[(2;0,4);(3;4,0)]);("0+2",[(4;0,0)])]);
/// ```
/// will create bdd with 5 variable lhs, id 0 with 3 levels defined by lhs equations and arrays of nodes
macro_rules! bdd {
    (
        $nvar:expr;
        $id:expr; [
            $((
                $lhs:expr,[
                    $((
                        $id_node:expr;
                        $e0:expr,
                        $e1:expr
                    ));*
                ]
            ))
            ;*
        ]
    ) => {
        $crate::soc::utils::build_bdd_from_spec(&mut utils::BddSpec::new(Id::new($id),
        [$($crate::soc::utils::LevelSpec::new($crate::soc::utils::vars(nom::types::CompleteStr(&$lhs)).expect("wrong format for lhs").1, [
            $($crate::soc::utils::NodeSpec::new(Id::new($id_node), Id::new($e0), Id::new($e1)))
            ,*].to_vec()))
        ,*].to_vec()),$nvar);
    }
}

#[macro_export]
/// Macro to generate systems :
///
/// ### Example :
///
/// ```text.
/// let system = system![bdd, bdd_2]?;
/// ```
/// will return a Result containing a System and fail if the bdds provided have a different nvar
macro_rules! system {
    [$($bdd:expr),*] => {
        $crate::soc::system::System::from_elem(vec![$($bdd),*])
    };
}

/// Custom type wrapping `usize` used for the ids of `node` inside a `Bdd` and
/// ids of `Bdd` inside a `System`. This is purely use for type safety and allows
/// for an easy modification of the storage type of the ids throughout the code (if one
/// wanted to change it to a `u32` or `u128` this would be the only place where a modification
/// needs to occur).

#[derive(Default, Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Id {
    val: usize,
}

impl Id {
    #[inline]
    pub fn new(val: usize) -> Id {
        Id { val }
    }
}

impl Deref for Id {
    type Target = usize;

    fn deref(&self) -> &Self::Target {
        &self.val
    }
}

impl Display for Id {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.val)
    }
}

#[cfg(test)]
mod test;
