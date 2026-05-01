//! Ordering module extracted from rustysd
//!
//! Provides service dependency ordering, cycle detection, and startup graph collection.
//! Based on rustysd's ordering system: https://github.com/KillingSpark/rustysd

#![allow(clippy::manual_retain)]
#![allow(clippy::let_and_return)]
#![allow(clippy::iter_nth_zero)]
#![allow(clippy::question_mark)]
use std::collections::HashMap;

/// Service identifier
#[derive(Clone, Eq, PartialEq, Hash, Debug, Ord, PartialOrd)]
pub struct UnitId {
    /// Service name
    pub name: String,
}

impl UnitId {
    /// Create a new UnitId
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }
}

impl PartialEq<str> for UnitId {
    fn eq(&self, other: &str) -> bool {
        self.name == other
    }
}

impl PartialEq<String> for UnitId {
    fn eq(&self, other: &String) -> bool {
        self.name == *other
    }
}

/// Service dependencies (bidirectional)
#[derive(Debug, Clone, Default)]
pub struct Dependencies {
    /// Services that this service wants (soft dependency)
    pub wants: Vec<UnitId>,
    /// Services that want this service (reverse of wants)
    pub wanted_by: Vec<UnitId>,
    /// Services that this service requires (hard dependency)
    pub requires: Vec<UnitId>,
    /// Services that require this service (reverse of requires)
    pub required_by: Vec<UnitId>,
    /// Services that should start before this one
    pub before: Vec<UnitId>,
    /// Services this service should start after
    pub after: Vec<UnitId>,
}

impl Dependencies {
    /// Get units that must start before this unit
    pub fn start_before_this(&self) -> Vec<UnitId> {
        self.after.clone()
    }

    /// Get units that can start concurrently with this unit
    pub fn start_concurrently_with_this(&self) -> Vec<UnitId> {
        let mut ids = Vec::new();
        ids.extend(self.wants.iter().cloned());
        ids.extend(self.requires.iter().cloned());
        let ids = ids
            .into_iter()
            .filter(|id| !self.after.contains(id))
            .collect();
        ids
    }

    /// Deduplicate dependencies
    pub fn dedup(&mut self) {
        self.wants.sort();
        self.wanted_by.sort();
        self.required_by.sort();
        self.before.sort();
        self.after.sort();
        self.requires.sort();
        self.wants.dedup();
        self.requires.dedup();
        self.wanted_by.dedup();
        self.required_by.dedup();
        self.before.dedup();
        self.after.dedup();
    }

    fn remove_from_vec(ids: &mut Vec<UnitId>, id: &UnitId) {
        while let Some(idx) = ids.iter().position(|e| e == id) {
            ids.remove(idx);
        }
    }

    /// Remove a unit from all dependency lists
    pub fn remove_id(&mut self, id: &UnitId) {
        Self::remove_from_vec(&mut self.wants, id);
        Self::remove_from_vec(&mut self.wanted_by, id);
        Self::remove_from_vec(&mut self.requires, id);
        Self::remove_from_vec(&mut self.required_by, id);
        Self::remove_from_vec(&mut self.before, id);
        Self::remove_from_vec(&mut self.after, id);
    }

    /// Check if this service comes after the named service
    pub fn comes_after(&self, name: &str) -> bool {
        for id in &self.after {
            if id.name == name {
                return true;
            }
        }
        false
    }

    /// Check if this service comes before the named service
    pub fn comes_before(&self, name: &str) -> bool {
        for id in &self.before {
            if id.name == name {
                return true;
            }
        }
        false
    }

    /// Check if the named service is required by this service
    pub fn requires(&self, name: &str) -> bool {
        for id in &self.requires {
            if id.name == name {
                return true;
            }
        }
        false
    }

    /// Check if the named service requires this service
    pub fn required_by(&self, name: &str) -> bool {
        for id in &self.required_by {
            if id.name == name {
                return true;
            }
        }
        false
    }

    /// Check if the named service is wanted by this service
    pub fn wants(&self, name: &str) -> bool {
        for id in &self.wants {
            if id.name == name {
                return true;
            }
        }
        false
    }

    /// Check if the named service wants this service
    pub fn wanted_by(&self, name: &str) -> bool {
        for id in &self.wanted_by {
            if id.name == name {
                return true;
            }
        }
        false
    }
}

/// Errors from dependency validation
#[derive(Debug, Eq, PartialEq)]
pub enum SanityCheckError {
    /// Generic error message
    Generic(String),
    /// Cycles detected in the dependency graph
    CirclesFound(Vec<Vec<UnitId>>),
}

impl std::fmt::Display for SanityCheckError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            SanityCheckError::Generic(msg) => write!(f, "{}", msg),
            SanityCheckError::CirclesFound(circles) => {
                write!(f, "Cycles found: ")?;
                for circle in circles {
                    write!(f, "{:?}", circle)?;
                }
                Ok(())
            }
        }
    }
}

impl std::error::Error for SanityCheckError {}

/// Validate that the unit dependencies form a valid DAG (no cycles)
///
/// Uses Kahn's algorithm for topological sorting
pub fn sanity_check_dependencies(
    unit_table: &HashMap<UnitId, Dependencies>,
) -> Result<(), SanityCheckError> {
    let mut root_ids = Vec::new();
    for unit in unit_table.values() {
        if unit.after.is_empty() {
            root_ids.push(unit.after.clone());
        }
    }

    let mut finished_ids = HashMap::new();
    let mut not_finished_ids: HashMap<_, _> =
        unit_table.keys().map(|id| (id.clone(), ())).collect();
    let mut circles = Vec::new();

    loop {
        let root_id = if not_finished_ids.is_empty() {
            break;
        } else {
            let root_id = not_finished_ids
                .keys()
                .filter(|id| {
                    let unit = unit_table.get(id).unwrap();
                    let in_degree = unit.after.iter().fold(0, |acc, id| {
                        if finished_ids.contains_key(id) {
                            acc
                        } else {
                            acc + 1
                        }
                    });
                    in_degree == 0
                })
                .nth(0);
            if let Some(id) = root_id {
                id.clone()
            } else {
                circles.push(not_finished_ids.keys().cloned().collect());
                break;
            }
        };

        let mut visited_ids = Vec::new();
        if let Err(SanityCheckError::CirclesFound(new_circles)) = search_backedge(
            &root_id,
            unit_table,
            &mut visited_ids,
            &mut finished_ids,
            &mut not_finished_ids,
        ) {
            circles.extend(new_circles)
        };
    }

    if circles.is_empty() {
        Ok(())
    } else {
        Err(SanityCheckError::CirclesFound(circles))
    }
}

fn search_backedge(
    id: &UnitId,
    unit_table: &HashMap<UnitId, Dependencies>,
    visited_ids: &mut Vec<UnitId>,
    finished_ids: &mut HashMap<UnitId, ()>,
    not_finished_ids: &mut HashMap<UnitId, ()>,
) -> Result<(), SanityCheckError> {
    if finished_ids.contains_key(id) {
        return Ok(());
    }

    if visited_ids.contains(id) {
        let circle_start_idx = visited_ids.iter().position(|i| i == id).unwrap_or(0);
        let circle_ids = visited_ids[circle_start_idx..].to_vec();
        for circleid in &circle_ids {
            finished_ids.insert(circleid.clone(), ());
            not_finished_ids.remove(circleid);
        }
        return Err(SanityCheckError::CirclesFound(vec![circle_ids]));
    }
    visited_ids.push(id.clone());

    let unit = unit_table.get(id).unwrap();
    for next_id in &unit.before {
        let res = search_backedge(
            next_id,
            unit_table,
            visited_ids,
            finished_ids,
            not_finished_ids,
        );
        if res.is_err() {
            return res;
        }
    }
    visited_ids.pop();
    finished_ids.insert(id.clone(), ());
    not_finished_ids.remove(id);

    Ok(())
}

/// Collect all units that need to be started to reach the target units
///
/// Extends ids_to_start with all dependencies recursively
pub fn collect_unit_start_subgraph(
    ids_to_start: &mut Vec<UnitId>,
    unit_table: &HashMap<UnitId, Dependencies>,
) {
    loop {
        let mut new_ids = Vec::new();
        for id in ids_to_start.iter() {
            let unit = match unit_table.get(id) {
                Some(u) => u,
                None => continue,
            };
            new_ids.extend(unit.start_before_this());
            new_ids.extend(unit.start_concurrently_with_this());
        }
        new_ids.sort();
        new_ids.dedup();
        new_ids = new_ids
            .into_iter()
            .filter(|id| !ids_to_start.contains(id))
            .collect();

        if new_ids.is_empty() {
            break;
        } else {
            ids_to_start.extend(new_ids);
        }
    }
}

/// Find units that can be started given current started state
pub fn find_startable_units(
    ids: &[UnitId],
    unit_table: &HashMap<UnitId, Dependencies>,
    started: &HashMap<UnitId, bool>,
) -> Vec<UnitId> {
    let mut startable = Vec::new();

    for id in ids {
        let unit = match unit_table.get(id) {
            Some(u) => u,
            None => continue,
        };

        let all_deps_satisfied = unit
            .after
            .iter()
            .all(|dep| started.get(dep).copied().unwrap_or(false));

        if all_deps_satisfied {
            startable.push(id.clone());
        }
    }
    startable
}

/// Add reverse dependency edges (wanted_by, required_by, before, after)
///
/// This ensures bidirectional tracking of dependencies
pub fn fill_dependencies(
    unit_table: &mut HashMap<UnitId, Dependencies>,
) -> Result<(), SanityCheckError> {
    let mut wanted_by = Vec::new();
    let mut required_by = Vec::new();
    let mut before = Vec::new();
    let mut after = Vec::new();

    for (id, deps) in unit_table.iter() {
        for wanted in &deps.wants {
            wanted_by.push((wanted.clone(), id.clone()));
        }
        for required in &deps.requires {
            required_by.push((required.clone(), id.clone()));
        }
        for before_id in &deps.before {
            after.push((id.clone(), before_id.clone()));
        }
        for after_id in &deps.after {
            before.push((id.clone(), after_id.clone()));
        }
    }

    for (wanted, wanting) in wanted_by {
        if let Some(unit) = unit_table.get_mut(&wanted) {
            unit.wanted_by.push(wanting);
        }
    }

    for (required, requiring) in required_by {
        if let Some(unit) = unit_table.get_mut(&required) {
            unit.required_by.push(requiring);
        }
    }

    for (before_id, after_id) in before {
        if let Some(unit) = unit_table.get_mut(&after_id) {
            unit.before.push(before_id);
        }
    }

    for (after_id, before_id) in after {
        if let Some(unit) = unit_table.get_mut(&before_id) {
            unit.after.push(after_id);
        }
    }

    for unit in unit_table.values_mut() {
        unit.dedup();
    }

    Ok(())
}

/// Find services that should be stopped when a service fails
///
/// Returns names of services that have the failed service in their required_by or wanted_by
pub fn propagate_failure(
    failed_service: &str,
    unit_table: &HashMap<UnitId, Dependencies>,
) -> Vec<String> {
    let mut to_stop = Vec::new();
    let failed_id = UnitId::new(failed_service);

    for (id, deps) in unit_table {
        if deps.required_by.contains(&failed_id) || deps.wanted_by.contains(&failed_id) {
            to_stop.push(id.name.clone());
        }
    }

    to_stop
}

