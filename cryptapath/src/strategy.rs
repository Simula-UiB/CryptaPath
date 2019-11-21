//! Module implementing the Solver and DroppingSolver traits from Crush and holding the
//! strategies used to solve a system of CRHS.  


use crush::{
    algebra,
    soc::{system::System, Id},
    solver::{Dependency, DroppingSolver, Independency, Solver},
};
use std::cell::Cell;
use std::io::Error;
use std::result::Result;

/// Describe the informations about a `Bdd` involved in a `NodeRankedDependency` or a `NodeRankedIndependency`.
#[derive(Clone, Debug)]
pub struct InvolvedBdd {
    /// id of the `Bdd` in the `System`.
    id: Id,
    /// Size of the levels part of the `NodeRankedDependency` or `NodeRankedIndependency`.
    levels: Vec<usize>,
    /// Total size of the BDD,
    total_size: usize,
    /// Index  of the levels part of the `NodeRankedDependency` or `NodeRankedIndependency`.
    involved_levels: Vec<usize>,
}

impl InvolvedBdd {
    /// Construct a new `InvolvedBdd` with the provided parameters.
    pub fn new(
        id: Id,
        levels: Vec<usize>,
        total_size: usize,
        involved_levels: Vec<usize>,
    ) -> InvolvedBdd {
        InvolvedBdd {
            id,
            levels,
            total_size,
            involved_levels,
        }
    }

    /// Return the `id` of the Bdd
    pub fn get_id(&self) -> Id {
        self.id
    }

    /// Return a `Vec` containing the index of the levels part of the `NodeRankedDependency` or `NodeRankedIndependency`.
    pub fn get_involved_levels(&self) -> &[usize] {
        &self.involved_levels
    }

    /// Return the amount of nodes in the BDD
    pub fn get_total_size(&self) -> usize {
        self.total_size
    }
}

/// NodeRankedDependency impl the Dependency traits and for the function `minimize_distance`
/// and `best_join_order` use the number of nodes involved in the depencdy as the metrics.
/// The join order is chosen by the amount of nodes we avoid and the distance is the amount of nodes
/// which will be involved in all the operations
#[derive(Clone, Debug)]
pub struct NodeRankedDependency {
    involved_bdds: Vec<InvolvedBdd>,
}

impl NodeRankedDependency {
    pub fn involved_bdds(&self) -> std::slice::Iter<InvolvedBdd> {
        self.involved_bdds.iter()
    }
}

impl Dependency for NodeRankedDependency {
    /// The score produced by minimize distance will be equal to the number of nodes in
    /// the levels that will be involved in at least one operation (swap or add) to resolve
    /// the dependency.
    fn minimize_distance(&self) -> usize {
        if self.involved_bdds.len() == 1 {
            return match self.involved_bdds[0].involved_levels.len() {
                1 => 0,
                _ => {
                    let start = self.involved_bdds[0].involved_levels[0];
                    let end = *self.involved_bdds[0].involved_levels.iter().last().unwrap();
                    self.involved_bdds[0]
                        .levels
                        .iter()
                        .skip(start)
                        .take(end)
                        .sum::<usize>()
                }
            };
        }
        let join_order = self.best_join_order();
        let mut score = 0;
        let start = *join_order.0[0];
        let end = **join_order.0.iter().last().unwrap();
        self.involved_bdds
            .iter()
            .for_each(|bdd| match *bdd.id as usize {
                a if start == a => {
                    score += bdd
                        .levels
                        .iter()
                        .skip(bdd.involved_levels[0])
                        .sum::<usize>()
                }
                a if end == a => {
                    score += bdd
                        .levels
                        .iter()
                        .take(*bdd.involved_levels.iter().last().unwrap())
                        .sum::<usize>()
                }
                _ => score += bdd.levels.iter().sum::<usize>(),
            });
        score
    }

    /// The best join order for a `NodeRankedDependency` is produced by finding the
    /// best bdd to put on top by finding the amount of nodes that we can
    /// avoid involving for each bdd and taking the bdd which has the biggest
    /// (the top levels will be avoided if the bdd is placed first in the join order
    /// while the bdd in middle will have to be traversed completely).
    /// Repeat the same process to find the last BDD and then all other BDD are joined
    /// randomly between the 2 picked.
    fn best_join_order(&self) -> (Vec<Id>, Vec<usize>) {
        if self.involved_bdds.len() == 1 {
            return (
                vec![self.involved_bdds[0].get_id()],
                self.involved_bdds[0].get_involved_levels().to_vec(),
            );
        }
        let mut dep = self.clone();
        let mut res = (Vec::new(), Vec::new());
        let start = dep.involved_bdds.iter().enumerate().fold(
            (0, 0),
            |(max_nodes_saved, id_start), (i, bdd)| {
                let nodes_saved = bdd
                    .levels
                    .iter()
                    .take(bdd.involved_levels[0])
                    .sum::<usize>();
                if nodes_saved > max_nodes_saved {
                    (nodes_saved, i)
                } else {
                    (max_nodes_saved, id_start)
                }
            },
        );
        let mut len_above = 0;
        let start = dep.involved_bdds.remove(start.1);
        res.0.push(start.get_id());
        res.1.append(&mut start.get_involved_levels().to_vec());
        len_above += start.levels.len();
        let end = dep.involved_bdds.iter().enumerate().fold(
            (0, 0),
            |(max_nodes_saved, id_start), (i, bdd)| {
                let nodes_saved = bdd
                    .levels
                    .iter()
                    .skip(*bdd.involved_levels.iter().last().unwrap())
                    .sum::<usize>();
                if nodes_saved > max_nodes_saved {
                    (nodes_saved, i)
                } else {
                    (max_nodes_saved, id_start)
                }
            },
        );
        let end = dep.involved_bdds.remove(end.1);
        let mut all_other = dep.involved_bdds.iter().map(|bdd| bdd.get_id()).collect();
        res.0.append(&mut all_other);
        res.0.push(end.get_id());
        for bdd in dep.involved_bdds.iter() {
            let involved_levels = bdd.get_involved_levels().to_vec();
            for level in involved_levels.iter() {
                res.1.push(level + len_above);
            }
            len_above += bdd.levels.len();
        }
        res.1.append(
            &mut end
                .get_involved_levels()
                .iter()
                .map(|level| level + len_above)
                .collect(),
        );
        res
    }

    /// Build the linear dependencies of the system.
    fn extract(system: &System) -> Vec<NodeRankedDependency> {
        let mut deps = Vec::new();
        let mut id_lhs = system.get_system_lhs();
        let mut lhs_concat = Vec::new();
        let mut id_levels_size = Vec::new();
        for bdd in id_lhs.iter_mut() {
            let mut levels = Vec::new();
            let total_size;
            {
                let bdd_object = system.get_bdd(bdd.0).unwrap().borrow();
                bdd_object
                    .iter_levels()
                    .for_each(|level| levels.push(level.get_nodes_len()));
                total_size = bdd_object.get_size();
            }
            // Removes the sink since iter_levels doesn't skip the last
            levels.pop();
            id_levels_size.push((bdd.0, levels, total_size));
            lhs_concat.append(&mut bdd.1);
        }
        let lin_dep = algebra::extract_linear_dependencies(matrix![lhs_concat]);

        for m_row in lin_dep.iter_rows() {
            let mut involved_bdds = Vec::new();
            let mut id_levels_size_iter = id_levels_size.iter();
            let mut bdd = id_levels_size_iter.next().unwrap();
            let mut bdd_start_range = 0;
            let mut bdd_end_range = bdd.1.len() - 1;
            let mut involved = Vec::new();
            for bit in m_row.iter_set_bits(..) {
                // for each bit (a bit is a level involved in the dep) :
                // check if the bit is in the range of the bdd (between its first and its last level)
                // if it is -> add it in the involved
                // if it is not -> update the range by proceeding to the next BDD, if the involved bdd wasn't
                // empty push it to the bdds of the dep
                // when all the row has been processed the bdds make one dep
                if bit <= bdd_end_range {
                    involved.push(bit - bdd_start_range);
                } else {
                    if !involved.is_empty() {
                        involved_bdds.push(InvolvedBdd::new(bdd.0, bdd.1.clone(), bdd.2, involved));
                        involved = Vec::new();
                    }
                    while bit > bdd_end_range {
                        bdd = id_levels_size_iter.next().unwrap();
                        let len = bdd.1.len();
                        bdd_start_range = bdd_end_range + 1;
                        bdd_end_range += len;
                    }
                    involved.push(bit - bdd_start_range);
                }
            }
            involved_bdds.push(InvolvedBdd::new(bdd.0, bdd.1.clone(), bdd.2, involved));
            deps.push(NodeRankedDependency { involved_bdds });
        }
        deps
    }
}

#[derive(Debug)]
struct BDDPatern {
    ids: Vec<Id>,
    deps: Vec<usize>,
    weigth: usize,
}

pub fn find_best_bdd_pattern_dep(deps: &[NodeRankedDependency]) -> Vec<NodeRankedDependency> {
    let mut best_deps = Vec::new();
    let mut patterns: Vec<BDDPatern> = Vec::new();
    deps.iter().enumerate().for_each(|(index_dep, dep)| {
        let (mut ids, weigth) = dep
            .involved_bdds()
            .fold((Vec::new(), 0), |(mut i, w), bdd| {
                i.push(bdd.get_id());
                (i, w + bdd.get_total_size())
            });
        ids.sort();
        if let Some(p) = patterns.iter_mut().find(|p| p.ids == ids) {
            p.deps.push(index_dep);
        } else {
            patterns.push(BDDPatern {
                ids,
                deps: vec![index_dep],
                weigth,
            });
        }
    });
    let (best_pattern, _) =
        patterns
            .iter()
            .enumerate()
            .fold((0, std::usize::MAX), |(best_p, min_weigth), (i, p)| {
                let w = p.weigth / p.deps.len();
                if w < min_weigth {
                    (i, w)
                } else {
                    (best_p, min_weigth)
                }
            });
    for i in patterns[best_pattern].deps.iter() {
        best_deps.push(deps[*i].clone())
    }
    best_deps
}

#[derive(Default)]
pub struct UpwardSolver {
    remaining: usize,
    solved: usize,
    max_reached: Cell<usize>,
}

impl UpwardSolver {
    pub fn new() -> UpwardSolver {
        Default::default()
    }

    pub fn improved_solve(&mut self, system: &mut System) -> Result<Vec<Vec<Option<bool>>>, Error> {
        Self::absorb_all_equations(system)?;
        let mut deps = NodeRankedDependency::extract(system);
        self.remaining = deps.len();
        while !deps.is_empty() {
            deps = find_best_bdd_pattern_dep(&deps);
            Self::resolve(self, system, Self::pick_best_dep(deps))?;
            self.solved += 1;
            Self::feedback(self, system);
            Self::absorb_all_equations(system)?;
            deps = NodeRankedDependency::extract(system);
            self.remaining = deps.len();
            Self::feedback(self, system);
        }
        Ok(system.get_solutions())
    }
}

impl Solver for UpwardSolver {
    fn feedback(&self, system: &System) {
        print!("\x1Bc");
        println!(
            "{} bdds remaining\n{} total nodes remaining\ntotal linear equations found {}\nsolved dependencies {}, {} remaining",
            system.iter_bdds().len(),
            system.get_size(),
            system.get_lin_bank_size(),
            self.solved,
            self.remaining,
        );
        let max_size = system.iter_bdds().fold(0, |size, bdd| {
            if bdd.1.borrow().get_size() > size {
                (bdd.1.borrow().get_size())
            } else {
                size
            }
        });
        println!("biggest bdd has {} nodes", max_size);
        let total_nodes = system
            .iter_bdds()
            .fold(0, |acc, bdd| acc + bdd.1.borrow().get_size());
        if total_nodes > self.max_reached.get() {
            self.max_reached.set(total_nodes);
        }
        println!(
            "max node reach 2**{}",
            (self.max_reached.get() as f64).log(2.0)
        );
    }
}

/// NodeRankedIndependency impl the Independency traits and for the function `minimize_distance`
/// and `best_join_order` use the number of nodes involved in the independency as the metric.
/// The join order is chosen by the amount of nodes we avoid and the distance is the amount of nodes
/// which will be involved in all the operations.
#[derive(Clone, Debug)]
pub struct NodeRankedIndependency {
    involved_bdds: Vec<InvolvedBdd>,
}

impl NodeRankedIndependency {
    pub fn involved_bdds(&self) -> std::slice::Iter<InvolvedBdd> {
        self.involved_bdds.iter()
    }
}

impl Independency for NodeRankedIndependency {
    /// The distance returned is equal to number of nodes in the levels
    /// that will be use to resolve the independency.
    fn minimize_distance(&self) -> usize {
        if self.involved_bdds.len() == 1 {
            let start = self.involved_bdds[0].involved_levels[0];
            return self.involved_bdds[0]
                .levels
                .iter()
                .skip(start)
                .sum::<usize>();
        }
        let join_order = self.best_join_order();
        let mut score = 0;
        let start = *join_order.0[0];
        // sum of nodes in all bdds except start + sum of nodes in the levels involved
        // in the first BDD
        self.involved_bdds
            .iter()
            .for_each(|bdd| match *bdd.id as usize {
                id if id == start => {
                    score += bdd
                        .levels
                        .iter()
                        .skip(bdd.involved_levels[0])
                        .sum::<usize>()
                }
                _ => score += bdd.levels.iter().sum::<usize>(),
            });
        score
    }

    /// The best join order for a `NodeRankedIndependency` is produced by finding the
    /// best bdd to put on top by finding the amount of nodes that we can
    /// avoid involving for each bdd and taking the bdd which has the biggest
    /// (the top levels will be avoided if the bdd is placed first in the join order
    /// while all the others bdds will have to be traversed completely).
    fn best_join_order(&self) -> (Vec<Id>, Vec<usize>) {
        if self.involved_bdds.len() == 1 {
            return (
                vec![self.involved_bdds[0].get_id()],
                self.involved_bdds[0].get_involved_levels().to_vec(),
            );
        }
        let mut indep = self.clone();
        let mut res = (Vec::new(), Vec::new());
        let start = indep.involved_bdds.iter().enumerate().fold(
            (0, 0),
            |(max_nodes_saved, id_start), (i, bdd)| {
                let nodes_saved = bdd
                    .levels
                    .iter()
                    .take(bdd.involved_levels[0])
                    .sum::<usize>();
                if nodes_saved > max_nodes_saved {
                    (nodes_saved, i)
                } else {
                    (max_nodes_saved, id_start)
                }
            },
        );
        let mut len_above = 0;
        let start = indep.involved_bdds.remove(start.1);
        res.0.push(start.get_id());
        res.1.append(&mut start.get_involved_levels().to_vec());
        len_above += start.levels.len();
        let mut all_other = indep.involved_bdds.iter().map(|bdd| bdd.get_id()).collect();
        res.0.append(&mut all_other);
        for bdd in indep.involved_bdds.iter() {
            let levels = bdd.get_involved_levels().to_vec();
            for level in levels.iter() {
                res.1.push(level + len_above);
            }
            len_above += bdd.levels.len();
        }
        res
    }

    /// Build the indepency for the system. The independencies for the variable contained
    /// in limit are not built. Each independency is a row of the transpose matrix representation
    /// of the entire system. Each independency therefore describe all the levels containing a specific variable.
    fn extract(system: &System, limit: Option<&[usize]>) -> Vec<NodeRankedIndependency> {
        let mut indeps = Vec::new();
        let mut id_lhs = system.get_system_lhs();
        let mut lhs_concat = Vec::new();
        let mut id_levels_size = Vec::new();
        for mut bdd in id_lhs.drain(..) {
            let mut levels = Vec::new();
            let total_size;
            {
                let bdd_object = system.get_bdd(bdd.0).unwrap().borrow();
                bdd_object
                    .iter_levels()
                    .for_each(|level| levels.push(level.get_nodes_len()));
                total_size = bdd_object.get_size();
            }
            //Removes the sink
            levels.pop();
            id_levels_size.push((bdd.0, levels, total_size));
            lhs_concat.append(&mut bdd.1);
        }
        let lin_indep = algebra::transpose(&matrix![lhs_concat]);
        for (var, m_row) in lin_indep.iter_rows().enumerate() {
            if limit.is_some() && limit.unwrap().contains(&var) {
                continue;
            }
            if m_row.iter_set_bits(..).next().is_none() {
                continue;
            }
            let mut involved_bdds = Vec::new();
            let mut id_levels_size_iter = id_levels_size.iter();
            let mut bdd = id_levels_size_iter.next().unwrap();
            let mut bdd_start_range = 0;
            let mut bdd_end_range = bdd.1.len() - 1;
            let mut involved = Vec::new();
            for bit in m_row.iter_set_bits(..) {
                // for each bit (a bit is a level involved in the dep) :
                // check if the bit is in the range of the bdd (between its first and its last level)
                // if it is -> add it in the involved
                // if it is not -> update the range by proceeding to the next BDD, if the involved bdd wasn't
                // empty push it to the bdds of the dep
                // when all the row has been processed the bdds make one dep
                if bit <= bdd_end_range {
                    involved.push(bit - bdd_start_range);
                } else {
                    if !involved.is_empty() {
                        involved_bdds.push(InvolvedBdd::new(bdd.0, bdd.1.clone(), bdd.2, involved));
                        involved = Vec::new();
                    }
                    while bit > bdd_end_range {
                        bdd = id_levels_size_iter.next().unwrap();
                        let len = bdd.1.len();
                        bdd_start_range = bdd_end_range + 1;
                        bdd_end_range += len;
                    }
                    involved.push(bit - bdd_start_range);
                }
            }
            involved_bdds.push(InvolvedBdd::new(bdd.0, bdd.1.clone(), bdd.2, involved));
            if involved_bdds.len() == 1 {
                indeps.push(NodeRankedIndependency { involved_bdds });
            }
        }
        indeps
    }
}

#[derive(Default)]
pub struct UpwardDroppingSolver {
    remaining: usize,
    solved: usize,
    dropped: usize,
    max_reached: Cell<usize>,
}

impl UpwardDroppingSolver {
    pub fn new() -> UpwardDroppingSolver {
        Default::default()
    }

    pub fn improved_solve(
        &mut self,
        system: &mut System,
        forbid_dropping: Option<&[usize]>,
    ) -> Result<Vec<Vec<Option<bool>>>, Error> {
        Self::absorb_all_equations(system)?;
        let mut deps = NodeRankedDependency::extract(system);
        let mut indeps = NodeRankedIndependency::extract(system, forbid_dropping);
        self.remaining = deps.len();
        while !deps.is_empty() {
            deps = find_best_bdd_pattern_dep(&deps);
            let (id_dep, min_distance_dep) = Self::pick_best_dep(&deps);
            let (id_indep, min_distance_indep) = Self::pick_best_indep(&indeps);
            if min_distance_indep < min_distance_dep {
                Self::indep_resolver(self, system, indeps[id_indep].best_join_order())?;
                self.dropped += 1;
            } else {
                Self::dep_resolver(self, system, deps[id_dep].best_join_order())?;
                self.solved += 1;
            }

            Self::feedback(self, system);
            Self::absorb_all_equations(system)?;
            deps = NodeRankedDependency::extract(system);
            indeps = NodeRankedIndependency::extract(system, forbid_dropping);
            self.remaining = deps.len();
            Self::feedback(self, system);
        }
        Ok(system.get_solutions())
    }
}

impl DroppingSolver for UpwardDroppingSolver {
    fn feedback(&self, system: &System) {
        print!( "\x1Bc");
        println!(
            
            "{} bdds remaining\n{} total nodes remaining\ntotal linear equations found {}\nsolved dependencies {}, {} remaining\ndropped variables {}",
            system.iter_bdds().len(),
            system.get_size(),
            system.get_lin_bank_size(),
            self.solved,
            self.remaining,
            self.dropped
        )
        ;
        let max_size = system.iter_bdds().fold(0, |size, bdd| {
            if bdd.1.borrow().get_size() > size {
                (bdd.1.borrow().get_size())
            } else {
                size
            }
        });
        println!( "biggest bdd has {} nodes", max_size);
        let total_nodes = system
            .iter_bdds()
            .fold(0, |acc, bdd| acc + bdd.1.borrow().get_size());
        if total_nodes > self.max_reached.get() {
            self.max_reached.set(total_nodes);
        }
        println!(
            "max node reach 2**{}",
            (self.max_reached.get() as f64).log(2.0)
        );
    }
}

pub fn execute_strategy_by_name(
    name: &str,
    system: &mut System,
    forbid_dropping: Option<&[usize]>,
) -> Option<Vec<Vec<Option<bool>>>> {
    match name {
        "no_drop" => {
            let mut solver = UpwardSolver::new();
            Some(solver.improved_solve(system).unwrap())
        }
        "drop" => {
            let mut solver = UpwardDroppingSolver::new();
            Some(solver.improved_solve(system, forbid_dropping).unwrap())
        }
        _ => None,
    }
}
