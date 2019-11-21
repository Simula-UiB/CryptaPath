//! Provide the traits to create solving strategies using the apis of `soc::System`.


use crate::soc::{system::System, Id};
use std::io::Error;
use std::result::Result;

/// Describe a dependency inside a `System` of `Bdd`. A `Dependency`
/// is defined as a collection of levels in a `System` which can be add to create a
/// 0-level (a level whose lhs is the all zero vector) that can be absorb. The levels can
/// be scattered accross multiples `Bdd` which will have to be join in order to resolve
/// the `Dependency`.
pub trait Dependency: Sized {
    /// Provide a way to estimate the cost of resolving the `Dependency`.
    /// Out of all dependencies, the one where the return of this function is the lowest should
    /// be the cheapest one to resolve.
    fn minimize_distance(&self) -> usize;
    /// Return the order in which the `Bdd`s involved in the `Dependency` should be joined,
    /// and the index of the levels to add to create a 0-level in the resulting `Bdd`.
    fn best_join_order(&self) -> (Vec<Id>, Vec<usize>);
    /// Extract all the `Dependency` in a given `System`
    fn extract(system: &System) -> Vec<Self>;
}

/// Describe an independency inside a `System` of `Bdd`. An independency is a
/// collections of level which contain every instance of a given variable. By adding a level of
/// this collection to all the other one can create a level which hold the only occurence of a
/// variable. This level can then be drop, losing the information to fix the value of this variable
/// when solving but making the `System` lighter. There is one `Independency` per variable in the
/// `System`.
pub trait Independency: Sized {
    /// Provide a way to estimate the cost of resolving the `Independency`.
    /// Out of all independencies, the one where the return of this function is the lowest should
    /// be the cheapest one to resolve.
    fn minimize_distance(&self) -> usize;
    /// Return the order in which the `Bdd`s involved in the `Independency` should be joined,
    /// and the index of the levels involved in the resulting `Bdd`.
    fn best_join_order(&self) -> (Vec<Id>, Vec<usize>);
    /// Extract all the `Independency` in a given `System` excluding those for which the variable
    /// is contained in `forbid_dropping`.
    fn extract(system: &System, forbid_dropping: Option<&[usize]>) -> Vec<Self>;
}

/// Describe a `Solver` as an object able to mutate a `System` in order
/// to remove all its linear dependencies and returning
/// solutions to the system of equations that it represents.
/// A `Solver` will only use the absorbtion methods, without using the
/// drop. We describe a solver through 4 methods:
///
/// - pick_best_dep which is a way to select the next `Dependency` to be resolved
///
/// - resolve which is a way to specify how will a given `Dependency` be remove from the `System`
///
/// - feedback which provide ongoing information to the user during the solving
///
/// - solve which act as an entry point and will call the other methods in a loop
/// until all `Dependency` have been removed
///
/// We provide default implementations for all of those methods.
pub trait Solver {
    /// Remove every linear dependency in a `System` using absorbtion and return the solutions
    fn solve<T: Dependency>(
        &mut self,
        system: &mut System,
    ) -> Result<Vec<Vec<Option<bool>>>, Error> {
        Self::absorb_all_equations(system)?;
        let mut deps = T::extract(system);
        while !deps.is_empty() {
            Self::resolve(self, system, Self::pick_best_dep(deps))?;
            Self::feedback(self, system);
            Self::absorb_all_equations(system)?;
            Self::feedback(self, system);
            deps = T::extract(system);
        }
        Ok(system.get_solutions())
    }

    /// Find the `Dependency` that should be resolved next and return the order in which
    /// the involved `Bdd`s should be joined and the index of the levels in the resulting
    /// joined `Bdd` that compose the dependency.
    fn pick_best_dep<T: Dependency>(deps: Vec<T>) -> (Vec<Id>, Vec<usize>) {
        let (id_dep, _) = deps.iter().enumerate().fold(
            (0, std::usize::MAX),
            |(id_dep, min_distance), (i, dep)| {
                if dep.minimize_distance() < min_distance {
                    (i, dep.minimize_distance())
                } else {
                    (id_dep, min_distance)
                }
            },
        );
        deps[id_dep].best_join_order()
    }

    /// Provide information about the solving process to the user.
    ///
    /// If you need information that are not contained in the `System` (ex: number of dependencies absorbed),
    /// the most easy way of getting them is to make them a field of your `Solver` and updating
    /// the fields during the solving.
    fn feedback(&self, system: &System) {
        print!("\x1Bc");
        println!(
            "{} bdds remaining\n{} total nodes remaining\ntotal linear equations found {}",
            system.iter_bdds().len(),
            system.get_size(),
            system.get_lin_bank_size()
        );
        let max_size = system.iter_bdds().fold(0, |size, bdd| {
            if bdd.1.borrow().get_size() > size {
                (bdd.1.borrow().get_size())
            } else {
                size
            }
        });
        println!("biggest bdd has {} nodes", max_size);
    }

    /// Describe the way a `Dependency` should be resolved.
    ///
    /// The `join_order` parameter should be the return value of `pick_best_dep`,
    /// that way you can chain the two functions.
    fn resolve(
        &self,
        system: &mut System,
        join_order: (Vec<Id>, Vec<usize>),
    ) -> Result<(), Error> {
        let mut keys_iter = join_order.0.iter();
        let bdd_root_id = keys_iter.next().unwrap();
        for key in keys_iter {
            system
                .join_bdds(*bdd_root_id, *key)
                .expect("should not crash when joining");
        }
        for i in (0..join_order.1.len() - 1).rev() {
            for j in (join_order.1[i] + 1..join_order.1[i + 1]).rev() {
                system.swap(*bdd_root_id, j, j + 1)?;
            }
            system.add(*bdd_root_id, join_order.1[i], join_order.1[i] + 1)?;
            if i != 0 {
                system.swap(*bdd_root_id, join_order.1[i], join_order.1[i] + 1)?;
            }
            Self::feedback(self, system);
        } 
        system.absorb(*bdd_root_id, join_order.1[0] + 1, false)?;
        Ok(())
    }

    /// Go through all BDDs and check for equation to absorb
    /// until there are no left. If when absorbing a BDD is reduced to
    /// its sink then we remove it from the system
    fn absorb_all_equations(system: &mut System) -> Result<(), Error> {
        let mut absorbed = true;
        while absorbed {
            absorbed = false;
            let ids = system
                .iter_bdds()
                .map(|bdd| *bdd.0)
                .collect::<Vec<Id>>();
            for id in ids.iter() {
                if system
                    .scan_absorb_lin_eqs(*id)
                    .expect("shouldn't crash when absorbing lin eq")
                    > 0
                {
                    absorbed = true;
                }
            }
            for id in ids.iter() {
                if system.get_bdd(*id)?.borrow().get_sink_level_index() == 0 {
                    system.pop_bdd(*id)?;
                }
            }
        }
        Ok(())
    }
}
/// Describe a `DroppingSolver` as an object able to mutate a `System` in order
/// to remove all its linear dependencies and returning
/// solutions to the system of equations that it represents.
///
/// A `DroppingSolver` will both use the absorbtion and the dropping methods, losing
/// the value of some variable voluntarily to simplify the resolution and focus
/// on the variables for which the value is the most important (typically the key or
/// a preimage).
/// We describe a dropping solver through 6 methods:
///
/// - pick_best_dep which is a way to select the next `Dependency` to be resolved
///
/// - dep_resolver which is a way to specify how will a given `Dependency` be remove from the `System`
///
/// - pick_best_indep which is a way to select the next `Independency` to be resolved
///
/// - indep_resolver which is a way to specify how will a given `Independency` be remove from the `System`
///
/// - feedback which provide ongoing information to the user during the solving
///
/// - solve which act as an entry point and will call the other methods in a loop
/// until all `Dependency` have been removed. Solve will also be responsible for choosing if it
/// should resolve the best `Dependency` or the best `Independency` next.
///
/// We provide default implementations for all of those methods.
pub trait DroppingSolver {
    /// Remove every linear dependency in a `System` using absorbtion and dropping and return the solutions.
    ///
    /// If `forbid_dropping` is `Some` the variable it contains should not be dropped. `solve` is responsible
    /// for choosing if an `Independency` or a `Dependency` should be resolved next, base on the
    /// `minimize_distance` result of the best `Independency` and `Dependency`.
    ///
    /// Not all possible drop have to be made as the purpose of dropping is only to make absorbing the
    /// dependencies faster, so we exit and get the solutions as soon as no dependencies are left
    /// in the `System`.
    fn solve<D: Dependency, I: Independency>(
        &mut self,
        system: &mut System,
        forbid_dropping: Option<&[usize]>,
    ) -> Result<Vec<Vec<Option<bool>>>, Error> {
        Self::absorb_all_equations(system)?;
        let mut deps = D::extract(system);
        let mut indeps = I::extract(system, forbid_dropping);
        while !deps.is_empty() {
            let (id_dep, min_distance_dep) = Self::pick_best_dep(&deps);
            let (id_indep, min_distance_indep) = Self::pick_best_indep(&indeps);
            if min_distance_indep < min_distance_dep {
                Self::indep_resolver(self, system, indeps[id_indep].best_join_order())?;
            } else {
                Self::dep_resolver(self, system, deps[id_dep].best_join_order())?;
            }
            Self::feedback(self, system);
            Self::absorb_all_equations(system)?;
            Self::feedback(self, system);
            deps = D::extract(system);
            indeps = I::extract(system, forbid_dropping);
        }
        Ok(system.get_solutions())
    }

    /// Describe the way an `Independency` should be resolved.
    ///
    /// The way we resolve an `Independency` is by adding a single
    /// level to every other level that contains a variable and finally
    /// dropping it.
    ///
    /// The `join_order` parameter should be the return value of `pick_best_indep`,
    /// that way you can chain the two functions.
    fn indep_resolver(
        &self,
        system: &mut System,
        join_order: (Vec<Id>, Vec<usize>),
    ) -> Result<(), Error> {
        let mut keys_iter = join_order.0.iter();
        let bdd_root_id = keys_iter.next().unwrap();
        for key in keys_iter {
            system
                .join_bdds(*bdd_root_id, *key)
                .expect("should not crash when joining");
        }
        for i in 0..join_order.1.len() - 1 {
            system.add(*bdd_root_id, join_order.1[i], join_order.1[i + 1])?;
            system.swap(*bdd_root_id, join_order.1[i + 1] - 1, join_order.1[i + 1])?;
            Self::feedback(self, system);
        }
        system.drop(*bdd_root_id, *join_order.1.last().unwrap())?;
        Self::feedback(self, system);
        Ok(())
    }

    /// Describe the way a `Dependency` should be resolved.
    ///
    /// The `join_order` parameter should be the return value of `pick_best_dep`,
    /// that way you can chain the two functions.
    fn dep_resolver(
        &self,
        system: &mut System,
        join_order: (Vec<Id>, Vec<usize>),
    ) -> Result<(), Error> {
        let mut keys_iter = join_order.0.iter();
        let bdd_root_id = keys_iter.next().unwrap();
        for key in keys_iter {
            system
                .join_bdds(*bdd_root_id, *key)
                .expect("should not crash when joining");
        }
        for i in (0..join_order.1.len() - 1).rev() {
            for j in (join_order.1[i] + 1..join_order.1[i + 1]).rev() {
                system.swap(*bdd_root_id, j, j + 1)?;
            }
            system.add(*bdd_root_id, join_order.1[i], join_order.1[i] + 1)?;
            if i != 0 {
                system.swap(*bdd_root_id, join_order.1[i], join_order.1[i] + 1)?;
            }
            Self::feedback(&self, system);
        }
        system.absorb(*bdd_root_id, join_order.1[0] + 1, false)?;
        Ok(())
    }

    /// Find the `Dependency` that should be resolved next and return the order in which
    /// the involved `Bdd`s should be joined and the index of the levels in the resulting
    /// joined `Bdd` that compose the dependency.
    fn pick_best_dep<T: Dependency>(deps: &[T]) -> (usize, usize) {
        deps.iter()
            .enumerate()
            .fold((0, std::usize::MAX), |(id_dep, min_distance), (i, dep)| {
                if dep.minimize_distance() < min_distance {
                    (i, dep.minimize_distance())
                } else {
                    (id_dep, min_distance)
                }
            })
    }

    /// Find the `Independency` that should be resolved next and return the order in which
    /// the involved `Bdd`s should be joined and the index of the levels in the resulting
    /// joined `Bdd` that compose the independency.
    fn pick_best_indep<T: Independency>(indeps: &[T]) -> (usize, usize) {
        indeps.iter().enumerate().fold(
            (0, std::usize::MAX),
            |(id_indep, min_distance), (i, indep)| {
                if indep.minimize_distance() < min_distance {
                    (i, indep.minimize_distance())
                } else {
                    (id_indep, min_distance)
                }
            },
        )
    }

    /// Go through all BDDs and check for equation to absorb
    /// until there are no left. If when absorbing a BDD is reduced to
    /// its sink then we remove it from the system
    fn absorb_all_equations(system: &mut System) -> Result<(), Error> {
        let mut absorbed = true;
        while absorbed {
            absorbed = false;
            let ids = system
                .iter_bdds()
                .map(|bdd| *bdd.0)
                .collect::<Vec<Id>>();
            for id in ids.iter() {
                if system
                    .scan_absorb_lin_eqs(*id)
                    .expect("shouldn't crash when absorbing lin eq")
                    > 0
                {
                    absorbed = true;
                }
            }
            for id in ids.iter() {
                if system.get_bdd(*id)?.borrow().get_sink_level_index() == 0 {
                    system.pop_bdd(*id)?;
                }
            }
        }
        Ok(())
    }

    /// Provide information about the solving process to the user.
    ///
    /// If you need information that are not contained in the `System` (ex: number of dependencies absorbed),
    /// the most easy way of getting them is to make them a field of your `DroppingSolver` and updating
    /// the fields during the solving.
    fn feedback(&self, system: &System) {
        print!("\x1Bc");
        println!(
            "{} bdds remaining\n{} total nodes remaining\ntotal linear equations found {}",
            system.iter_bdds().len(),
            system.get_size(),
            system.get_lin_bank_size(),
        );
        let max_size = system.iter_bdds().fold(0, |size, bdd| {
            if bdd.1.borrow().get_size() > size {
                (bdd.1.borrow().get_size())
            } else {
                size
            }
        });
        println!("biggest bdd has {} nodes", max_size);
    }
}
