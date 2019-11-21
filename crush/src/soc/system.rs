//! A `System` is composed of a collections of `Bdd` (stored in a HashMap with their id
//! as key), a `LinBank` holding the `LinEq` found during the resolution and an `nvar` which
//! indicate the total number of variables present initially in the system of equations.
//!
//! This object will be mutated through it's different methods (fix, drop, add, swap, absorb, scan)
//! in order to remove all the linear dependencies among the levels of the different `Bdd`s so
//! the solutions to the system of equations it represents can be extracted.

use crate::algebra;
use crate::soc::{
    bdd::{Bdd, LinEq},
    Id,
};
use crate::AHashMap;

use std::cell::RefCell;
use std::fmt;
use std::io::{self, Error, ErrorKind};
use std::result::Result;
use vob::Vob;

/// A system of Bdds providing a number of methods to interact safely with the Bdds it contains
#[derive(Default)]
pub struct System {
    bdds: AHashMap<Id, RefCell<Bdd>>,
    nvar: usize,
    lin_bank: LinBank,
}

/// `LinBank` is the structure holding the valid linear equations
/// found while solving the system using the `scan_absorb_lin_eqs`
/// function.
///
/// All linear equations should be linearly independent from each
/// other before being push in the LinBank.
///
/// The way we ensure this is by adding to the incoming
/// linear equation all the equations already in the bank
/// where their hightest set bit is also set in the incoming
/// equation.
///
/// ##Example :
///
/// bank is `[{[1011], true}]`
///
/// -> want to push to the bank `{[0100], true}`
///
/// no issue, becomes `[{[1011],true},{[0100],true}]`
///
/// -> want to push v{[0111],true}`
///
/// 2nd bit is set in the incoming equation and is highest bit of the second equation of the bank
///
/// incoming equation become `{[0011],false}` before being push in the bank
///
/// Bank is now `[{[1011],true},{[0100],true},{[0011],false}]`
///
/// -> want to push `{[1100],true}`
///
/// first bit is set in incoming and is highest bit of first eq of the bank
///
/// incoming equation become `{[0111],false}`
///
/// second bit is set in incoming and is highest bit of second eq of the bank
///
/// incoming equation become `{[0011],true}`
///
/// third bit is set in incoming and is highest bit of third eq of the bank
///
/// incoming equation become `{[0000],false}` -> all zero vector, non independant
///
/// pushing is cancelled

#[derive(Default, Clone)]
struct LinBank {
    lin_eqs: Vec<LinEq>,
}

impl System {
    /// Construct a new System with default parameters
    pub fn new() -> System {
        Default::default()
    }

    /// Construct a `System` from a `Vec` of `Bdd` using the `nvar` of the first `Bdd`
    /// as its `nvar`.
    ///
    /// Will return an `Error` if all Bdds don't have the same `nvar`.
    pub fn from_elem(bdds: Vec<Bdd>) -> Result<System, Error> {
        let mut bdds = bdds;
        let mut sys = System::new();
        if bdds.is_empty() {
            return Err(Error::new(ErrorKind::InvalidInput, "Empty vec"));
        }
        sys.nvar = bdds[0].get_nvar_size();
        for bdd in bdds.drain(..) {
            sys.push_bdd(bdd)?
        }
        Ok(sys)
    }

    /// Set `nvar` of the `System`
    pub fn set_nvar(&mut self, nvar: usize) {
        self.nvar = nvar;
    }

    /// Get `nvar` of the `System`
    pub fn get_nvar(&self) -> usize {
        self.nvar
    }

    /// Push a `Bdd` in the system.
    ///
    /// Return an `Error` if the `nvar` of the `Bdd` is different from the `nvar` of the `System`, or
    /// if a `Bdd` with the same `id` was already present in the system
    pub fn push_bdd(&mut self, bdd: Bdd) -> Result<(), Error> {
        if bdd.get_nvar_size() != self.nvar {
            return Err(Error::new(
                ErrorKind::InvalidData,
                "Bdd have different nvar size from system",
            ));
        }
        if self.get_bdd(bdd.get_id()).is_ok() {
            return Err(Error::new(
                ErrorKind::InvalidData,
                "A Bdd with the same id is already in the system",
            ));
        }
        self.bdds.insert(bdd.get_id(), RefCell::new(bdd));
        Ok(())
    }

    /// Return a reference to the `Bdd` which `id` is equal to `bdd_id`.
    ///
    /// Will return an `Error` if there is no `Bdd` matching this condition.
    pub fn get_bdd(&self, bdd_id: Id) -> Result<&RefCell<Bdd>, Error> {
        match self.bdds.get(&bdd_id) {
            Some(bdd) => Ok(bdd),
            None => Err(Error::new(
                ErrorKind::InvalidData,
                format!("id {} not present in system", bdd_id),
            )),
        }
    }

    /// Split the `System` into 2 `System` removing from self all `Bdd` whose ids
    /// are contains in `ids` and returning a new `System` made of those `Bdd`.
    ///
    /// Will return an `Error` if one `Id` in `ids` doesn't match any `Bdd` in the `system`.
    pub fn split(&mut self, ids: &[Id]) -> Result<System, Error> {
        let mut bdds = Vec::with_capacity(ids.len());
        for id in ids {
            bdds.push(self.pop_bdd(*id)?);
        }
        let mut sys = System::from_elem(bdds)?;
        sys.lin_bank = self.lin_bank.clone();
        Ok(sys)
    }

    /// Merge a `System` into the current one by pushing all the non-empty `Bdd`
    /// in the `System` and all the `LinEq` of the `LinBank`.
    ///
    /// Will return an error if one of the `Bdd` has a different `nvar` from the `System`.
    pub fn merge(&mut self, system: &mut System) -> Result<(()), Error> {
        for bdd in system.drain_bdds() {
            // TODO -> error handling should take into account middle crash and rollback system to its initial state
            // to avoid half merging if one bdd have a different nvar
            if bdd.1.borrow().get_levels_size() > 1 {
                self.push_bdd(bdd.1.into_inner())?;
            }
        }
        for lin_eq in system.lin_bank.lin_eqs.drain(..) {
            self.push_lin_eq_to_lin_bank(lin_eq);
        }
        Ok(())
    }

    /// Join the two `Bdd` of the specified ids.
    ///
    /// The `bdd_1_id` will be the `id` of the resulting `Bdd`
    ///
    /// Returns the `bdd_1_id` if successfull, or an `Error` if
    /// `bdd_id_1` and `bdd_id_2` are equals or one is not found in the
    /// `System`.
    pub fn join_bdds(&mut self, bdd_1_id: Id, bdd_2_id: Id) -> Result<Id, Error> {
        if bdd_1_id == bdd_2_id {
            return Err(Error::new(
                ErrorKind::InvalidData,
                "bdd_1_id is equal to bdd_2_id",
            ));
        }
        let bdd_1 = self.get_bdd(bdd_1_id)?;
        let bdd_2 = self.get_bdd(bdd_2_id)?;
        let sink_level_id = bdd_1.borrow().get_sink_level_index();
        for level in bdd_2.borrow_mut().drain_levels() {
            bdd_1.borrow_mut().add_existing_level(level)
        }
        bdd_1.borrow_mut().merge_sink_source(sink_level_id);
        self.bdds.remove(&bdd_2_id);
        Ok(bdd_1_id)
    }

    /// Performs a `swap` operation on the `Bdd` with the `id` specified between the 2 level indexes given.
    ///
    /// Returns an `Error` if `level_index_above` is not directly above `level_index_below`, if
    /// `level_index_below` is out of the range of the levels the `Bdd`, or if `bdd_id` is not found in the `System`..
    pub fn swap(
        &mut self,
        bdd_id: Id,
        level_index_above: usize,
        level_index_below: usize,
    ) -> Result<(), Error> {
        if level_index_below != level_index_above + 1 {
            return Err(Error::new(
                ErrorKind::InvalidData,
                "Level 1 is not directly above Level 2",
            ));
        }
        let bdd = self.get_bdd(bdd_id)?;
        if level_index_below >= bdd.borrow().get_sink_level_index() {
            return Err(Error::new(ErrorKind::InvalidData, "Out of range of levels"));
        }
        bdd.borrow_mut().swap(level_index_above, level_index_below);
        Ok(())
    }

    /// Performs a `add` operation on the `Bdd` with the `id` specified between the 2 level indexes given.
    ///
    /// Returns an `Error` if `level_index_above` is not directly above `level_index_below`, if
    /// `level_index_below` is out of the range of the levels the `Bdd`, or if `bdd_id` is not found in the `System`.
    pub fn add(
        &mut self,
        bdd_id: Id,
        level_index_above: usize,
        level_index_below: usize,
    ) -> Result<(), Error> {
        if level_index_above >= level_index_below {
            return Err(Error::new(
                ErrorKind::InvalidData,
                "Level above is not above Level below",
            ));
        }
        let bdd = self.get_bdd(bdd_id)?;
        if level_index_below >= bdd.borrow().get_sink_level_index() {
            return Err(Error::new(
                ErrorKind::InvalidData,
                format!(
                    "Out of range of levels : trying to add on level {}, sink level is {}",
                    level_index_below,
                    bdd.borrow().get_sink_level_index()
                ),
            ));
        }
        bdd.borrow_mut().add(level_index_above, level_index_below);
        Ok(())
    }

    /// Performs an `absorb` operation on the `Bdd` with the `id` specified on `level_index` and along the edge specified.
    ///
    /// Returns an `Error` if `level_index` is out of the range of the levels the `Bdd`, or
    /// if `bdd_id` is not found in the `System`.
    pub fn absorb(&mut self, bdd_id: Id, level_index: usize, edge: bool) -> Result<(), Error> {
        let bdd = self.get_bdd(bdd_id)?;
        if level_index >= bdd.borrow().get_sink_level_index() {
            return Err(Error::new(
                ErrorKind::InvalidData,
                format!(
                    "Out of range of levels : trying to absorb {}, sink level is {}",
                    level_index,
                    bdd.borrow().get_sink_level_index()
                ),
            ));
        }
        bdd.borrow_mut().absorb(level_index, edge);
        Ok(())
    }

    /// Performs a `drop` operation on the `Bdd` with the `id` specified on `level_index`.
    ///
    /// Returns an `Error` if `level_index` is out of the range of the levels the `Bdd`, or
    /// if `bdd_id` is not found in the `System`.
    pub fn drop(&mut self, bdd_id: Id, level_index: usize) -> Result<(), Error> {
        let bdd = self.get_bdd(bdd_id)?;
        if level_index >= bdd.borrow().get_sink_level_index() {
            return Err(Error::new(
                ErrorKind::InvalidData,
                format!(
                    "Out of range of levels : trying to drop {}, sink level is {}",
                    level_index,
                    bdd.borrow().get_sink_level_index()
                ),
            ));
        }
        bdd.borrow_mut().drop(level_index);
        Ok(())
    }

    /// Fix the of a linear combination of variables in the `System` by adding a new LinEq to the LinBank.
    ///
    /// `lhs` contain all the variable of the left hand side of the equation
    /// and `rhs` is the right hand side of the equation.
    ///
    /// ```text.
    /// fix(vec![1,2,3], true) -> x1 + x2 + x3 = 1;
    /// ```
    ///
    /// Return an `Error` if the fix was not linearly independant from the LinBank.
    pub fn fix(&mut self, lhs: Vec<usize>, rhs: bool) -> Result<(), io::Error> {
        let mut lhs_as_vob = Vob::new();
        lhs_as_vob.resize(self.nvar, false);
        for var in lhs.iter() {
            lhs_as_vob.set(*var, true);
        }
        let lin_eq = LinEq::new(lhs_as_vob, rhs);
        match self.push_lin_eq_to_lin_bank(lin_eq) {
            Some(_) => Ok(()),
            None => Err(Error::new(
                ErrorKind::InvalidData,
                "linear equation non linearly independant from current LinBank",
            )),
        }
    }

    /// Scan the `Bdd` of `bdd_id` for `LinEq` and push the `LinEq`s found to the `LinBank`
    ///
    /// Returns the number of `LinEq` correctly absorbed or an `Error` if `bdd_id` is not in the
    /// `System`.
    pub fn scan_absorb_lin_eqs(&mut self, bdd_id: Id) -> Result<usize, io::Error> {
        let mut absorbed = 0;
        let bdd = self.get_bdd(bdd_id)?;
        let mut lin_eqs = bdd.borrow_mut().scan_absorb_lin_eq();
        for lin_eq in lin_eqs.drain(..) {
            if self.push_lin_eq_to_lin_bank(lin_eq).is_some() {
                absorbed += 1;
            }
        }
        Ok(absorbed)
    }

    /// Attempt to push the `LinEq` to the `LinBank` and if successfull remove the higher
    /// variable of the  modified `LinEq` from the whole `System`.
    ///
    /// Return `Some(modified lin_eq)` if successfull or `None` if `lin_eq` was not linearly
    /// independant from the `LinBank`.
    fn push_lin_eq_to_lin_bank(&mut self, lin_eq: LinEq) -> Option<LinEq> {
        match self.lin_bank.push_lin_eq(lin_eq) {
            Some(eq) => {
                let var = eq.get_lhs_max_set_bit().unwrap();
                for bdd in self.bdds.iter_mut() {
                    bdd.1.borrow_mut().replace_var_in_bdd(var, &eq);
                }
                Some(eq)
            }
            None => None,
        }
    }

    /// Get the number of nodes inside the `System`.
    pub fn get_size(&self) -> usize {
        self.bdds
            .iter()
            .fold(0, |acc, bdd| acc + bdd.1.borrow().get_size())
    }

    /// Iterate over the `bdds` of the `System`.
    pub fn iter_bdds(&self) -> std::collections::hash_map::Iter<Id, RefCell<Bdd>> {
        self.bdds.iter()
    }

    /// Drain over the `bdds` of the `System`.
    pub fn drain_bdds(&mut self) -> std::collections::hash_map::Drain<Id, RefCell<Bdd>> {
        self.bdds.drain()
    }

    /// Remove the `Bdd` of given index `bdd_id` from the `System` and returns it.
    ///
    /// Return an Error if `bdd_id` is not in the `System`.
    pub fn pop_bdd(&mut self, bdd_id: Id) -> Result<Bdd, io::Error> {
        match self.bdds.remove(&bdd_id) {
            Some(bdd_ref) => Ok(bdd_ref.into_inner()),
            None => Err(Error::new(
                ErrorKind::InvalidData,
                format!("id {} not present in system", *bdd_id),
            )),
        }
    }

    /// Return a `Vec` of tuples containing the ids and aggregated lhs of all `Bdd`s in the `System`.
    pub fn get_system_lhs(&self) -> Vec<(Id, Vec<Vob>)> {
        let mut system_lhs = Vec::new();
        for bdd in self.bdds.iter() {
            system_lhs.push((*bdd.0, bdd.1.borrow().get_lhs()));
        }
        system_lhs
    }

    /// Return the solutions to the `System` using the `LinBank` and the paths in the
    /// remaining BDDs. If multiple BDDs are still in the system it will join all of them to
    /// find the solutions.
    ///
    /// Will use the `algebra::solve_linear_system` to find the different solutions.
    pub fn get_solutions(&mut self) -> Vec<Vec<Option<bool>>> {
        let keys: Vec<Id> = self.bdds.keys().cloned().collect();
        let remaining_id = match keys.len() {
            // everything in linbank
            0 => {
                let lhs = self.lin_bank.get_lhs();
                let rhs = self.lin_bank.get_rhs();
                return vec![algebra::solve_linear_system(matrix![lhs], rhs)];
            }
            // only one BDD left
            1 => keys[0],
            // multiple BDD, join everything first
            _ => {
                for key in 1..keys.len() {
                    self.join_bdds(keys[0], keys[key]).unwrap();
                }
                keys[0]
            }
        };
        let paths = self
            .get_bdd(remaining_id)
            .unwrap()
            .borrow()
            .get_all_valid_path();
        let mut solutions = Vec::new();
        for path in paths {
            let mut lin_bank = self.lin_bank.clone();
            for eq in path {
                lin_bank.push_lin_eq(eq);
            }
            solutions.push(algebra::solve_linear_system(
                matrix![lin_bank.get_lhs()],
                lin_bank.get_rhs(),
            ));
        }
        solutions
    }

    /// Return the number of `LinEq` in the `LinBank`.
    pub fn get_lin_bank_size(&self) -> usize {
        self.lin_bank.lin_eqs.len()
    }
}

impl fmt::Debug for System {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "Full system")?;
        if self.bdds.is_empty() {
            writeln!(f, "No BDDs on this system")?;
        }
        for bdd in self.bdds.iter() {
            writeln!(f, "{:?}", bdd)?;
        }
        writeln!(f, "LinBank\n{:?}", self.lin_bank)?;
        Ok(())
    }
}

impl LinBank {
    /// Push the `LinEq` in the `LinBank` if `lin_eq` is linearly independent
    /// from the equations already in the `LinBank`.
    ///
    /// Perform the verification by adding (see `LinBank` doc).
    ///
    /// Return `Some(modified lin_eq)` if the lin_eq was pushed
    /// and `None` if it wasn't.
    pub fn push_lin_eq(&mut self, mut lin_eq: LinEq) -> Option<LinEq> {
        for lin_bank_eq in self.lin_eqs.iter() {
            if lin_eq
                .get_lhs()
                .get(lin_bank_eq.get_lhs_max_set_bit().unwrap())
                .unwrap()
            {
                lin_eq.add_lin_eq(&lin_bank_eq)
            }
        }
        match lin_eq.get_lhs_max_set_bit() {
            Some(_) => {
                self.lin_eqs.push(lin_eq.clone());
                Some(lin_eq)
            }
            None => None,
        }
    }

    /// Return a copy of all the left hand side of the equations inside the `LinBank`
    pub fn get_lhs(&self) -> Vec<Vob> {
        self.lin_eqs.iter().map(|lin_eq| lin_eq.get_lhs()).collect()
    }

    /// Return a `Vob` containing all the right hand side of the equations inside the `LinBank`
    pub fn get_rhs(&self) -> Vob {
        let mut rhs = Vob::from_elem(self.lin_eqs.len(), false);
        for (i, lin_eq) in self.lin_eqs.iter().enumerate() {
            if lin_eq.get_rhs() {
                rhs.set(i, true);
            }
        }
        rhs
    }
}

impl fmt::Debug for LinBank {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "LinBank")?;
        if self.lin_eqs.is_empty() {
            writeln!(f, "No lin dep in bank")?;
        }
        for lin_dep in self.lin_eqs.iter() {
            writeln!(f, "{:?}", lin_dep)?;
        }
        Ok(())
    }
}
