// Copyright (C) 2026 Antoine ANCEAU
//
// This file is part of navaltoolbox.
//
// navaltoolbox is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

//! Loading condition dataclasses.
//!
//! Defines `MassCategory`, `MassItem`, and `LoadingCondition` for grouping
//! multiple masses and tank fill overrides into a single configuration.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::vessel::Vessel;

// ============================================================================
// MassCategory
// ============================================================================

/// Category of a mass item.
///
/// Used for classification and reporting. Sub-categories of deadweight
/// (consumables, ballast, stores…) may be added in a future version.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MassCategory {
    /// Ship structure, machinery, and permanent equipment.
    Lightship,
    /// Variable loads: cargo, stores, crew, consumables, ballast, etc.
    Deadweight,
    /// Uncategorized mass item.
    Other,
}

impl Default for MassCategory {
    fn default() -> Self {
        MassCategory::Other
    }
}

impl std::fmt::Display for MassCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MassCategory::Lightship => write!(f, "Lightship"),
            MassCategory::Deadweight => write!(f, "Deadweight"),
            MassCategory::Other => write!(f, "Other"),
        }
    }
}

// ============================================================================
// MassItem
// ============================================================================

/// A single mass item with name, mass, position, and optional category.
///
/// Represents one element in a loading condition's weight inventory
/// (e.g. lightship, crew, provisions, equipment).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MassItem {
    /// Name identifier for the mass item.
    pub name: String,
    /// Mass in kg.
    pub mass: f64,
    /// Center of gravity [LCG, TCG, VCG] in meters.
    pub cog: [f64; 3],
    /// Classification category.
    #[serde(default)]
    pub category: MassCategory,
}

impl MassItem {
    /// Creates a new mass item with default category (`Other`).
    pub fn new(name: &str, mass: f64, cog: [f64; 3]) -> Self {
        Self {
            name: name.to_string(),
            mass,
            cog,
            category: MassCategory::default(),
        }
    }

    /// Sets the category (builder pattern).
    pub fn with_category(mut self, category: MassCategory) -> Self {
        self.category = category;
        self
    }
}

// ============================================================================
// LoadingCondition
// ============================================================================

/// A complete loading condition.
///
/// Groups multiple mass items and tank fill overrides into a single
/// configuration that can be applied to a [`Vessel`] for hydrostatic
/// and stability calculations.
///
/// # Usage
///
/// ```rust,ignore
/// use navaltoolbox::{LoadingCondition, MassItem, MassCategory};
///
/// let mut lc = LoadingCondition::new("Departure — Full Load");
/// lc.add_mass(MassItem::new("Lightship", 5_000_000.0, [45.0, 0.0, 4.5])
///     .with_category(MassCategory::Lightship));
/// lc.add_mass(MassItem::new("Crew", 3_000.0, [35.0, 0.0, 8.0]));
/// lc.set_tank_fill_percent("FO_1P", 95.0);
///
/// lc.apply(&mut vessel);
/// let (displacement, cog) = lc.resolve(&vessel);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadingCondition {
    /// Name of the loading condition.
    pub name: String,
    /// List of mass items.
    masses: Vec<MassItem>,
    /// Tank fill overrides: tank_name -> fill_level (0.0 to 1.0).
    /// Only tanks listed here will have their fill level changed
    /// when `apply()` is called. Other tanks keep their current state.
    tank_fills: HashMap<String, f64>,
}

impl LoadingCondition {
    /// Creates a new empty loading condition.
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            masses: Vec::new(),
            tank_fills: HashMap::new(),
        }
    }

    // =========================================================================
    // Mass Management
    // =========================================================================

    /// Adds a mass item to the loading condition.
    pub fn add_mass(&mut self, item: MassItem) {
        self.masses.push(item);
    }

    /// Convenience: adds a mass item with name, mass, and cog (category = Other).
    pub fn add_mass_simple(&mut self, name: &str, mass: f64, cog: [f64; 3]) {
        self.masses.push(MassItem::new(name, mass, cog));
    }

    /// Removes the first mass item with the given name.
    /// Returns true if a mass was removed, false if not found.
    pub fn remove_mass(&mut self, name: &str) -> bool {
        if let Some(idx) = self.masses.iter().position(|m| m.name == name) {
            self.masses.remove(idx);
            true
        } else {
            false
        }
    }

    /// Returns a reference to the list of mass items.
    pub fn masses(&self) -> &[MassItem] {
        &self.masses
    }

    /// Returns the number of mass items.
    pub fn num_masses(&self) -> usize {
        self.masses.len()
    }

    // =========================================================================
    // Tank Fill Overrides
    // =========================================================================

    /// Sets a tank fill override by fill level (0.0 to 1.0).
    ///
    /// The value is clamped to [0.0, 1.0].
    pub fn set_tank_fill(&mut self, tank_name: &str, fill_level: f64) {
        self.tank_fills
            .insert(tank_name.to_string(), fill_level.clamp(0.0, 1.0));
    }

    /// Sets a tank fill override by percentage (0 to 100).
    pub fn set_tank_fill_percent(&mut self, tank_name: &str, fill_percent: f64) {
        self.set_tank_fill(tank_name, fill_percent / 100.0);
    }

    /// Removes a tank fill override.
    /// Returns true if the override existed, false otherwise.
    pub fn remove_tank_fill(&mut self, tank_name: &str) -> bool {
        self.tank_fills.remove(tank_name).is_some()
    }

    /// Returns a reference to the tank fill overrides.
    pub fn tank_fills(&self) -> &HashMap<String, f64> {
        &self.tank_fills
    }

    /// Returns the number of tank fill overrides.
    pub fn num_tank_overrides(&self) -> usize {
        self.tank_fills.len()
    }

    // =========================================================================
    // Application & Calculation
    // =========================================================================

    /// Applies tank fill overrides to the vessel's tanks.
    ///
    /// Only tanks whose names appear in `tank_fills` are modified.
    /// Other tanks keep their current fill level.
    pub fn apply(&self, vessel: &mut Vessel) {
        for (tank_name, &fill_level) in &self.tank_fills {
            if let Some(tank) = vessel.get_tank_by_name(tank_name) {
                tank.write().unwrap().set_fill_level(fill_level);
            }
        }
    }

    /// Calculates the total displacement (masses + tank fluid masses).
    ///
    /// Must be called after `apply()` so that tank fill levels are current.
    pub fn total_displacement(&self, vessel: &Vessel) -> f64 {
        let masses_total: f64 = self.masses.iter().map(|m| m.mass).sum();
        let tanks_total: f64 = vessel.get_total_tanks_mass();
        masses_total + tanks_total
    }

    /// Calculates the combined center of gravity (mass-weighted average
    /// of all mass items + all tank fluid CoGs from the vessel).
    ///
    /// Must be called after `apply()` so that tank fill levels are current.
    pub fn total_cog(&self, vessel: &Vessel) -> [f64; 3] {
        let total_disp = self.total_displacement(vessel);
        if total_disp <= 0.0 {
            return [0.0, 0.0, 0.0];
        }

        let mut moment = [0.0f64; 3];

        // Mass items
        for m in &self.masses {
            moment[0] += m.mass * m.cog[0];
            moment[1] += m.mass * m.cog[1];
            moment[2] += m.mass * m.cog[2];
        }

        // Tank fluids from vessel
        for tank_arc in vessel.tanks() {
            let tank = tank_arc.read().unwrap();
            let mass = tank.fluid_mass();
            if mass > 0.0 {
                let cog = tank.center_of_gravity();
                moment[0] += mass * cog[0];
                moment[1] += mass * cog[1];
                moment[2] += mass * cog[2];
            }
        }

        [
            moment[0] / total_disp,
            moment[1] / total_disp,
            moment[2] / total_disp,
        ]
    }

    /// Convenience: returns `(total_displacement, total_cog)` in a single call.
    ///
    /// Must be called after `apply()` so that tank fill levels are current.
    pub fn resolve(&self, vessel: &Vessel) -> (f64, [f64; 3]) {
        (self.total_displacement(vessel), self.total_cog(vessel))
    }

    // =========================================================================
    // Serialization
    // =========================================================================

    /// Serializes the loading condition to a JSON string.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Deserializes a loading condition from a JSON string.
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// Serializes the loading condition to a JSON file.
    pub fn to_json_file(&self, path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
        let json = self.to_json()?;
        std::fs::write(path, json)?;
        Ok(())
    }

    /// Deserializes a loading condition from a JSON file.
    pub fn from_json_file(path: &std::path::Path) -> Result<Self, Box<dyn std::error::Error>> {
        let json = std::fs::read_to_string(path)?;
        let lc = Self::from_json(&json)?;
        Ok(lc)
    }
}
