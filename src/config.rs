use serde::{Deserialize, Serialize};
use crate::Float;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub domain: DomainConfig,
    pub physics: PhysicsConfig,
    pub simulation: SimulationConfig,
    pub output: OutputConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainConfig {
    pub nx: usize,
    pub ny: usize,
    pub nz: usize,
    pub dx: Float,
    pub dy: Float,
    pub dz: Float,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhysicsConfig {
    pub reynolds_number: Float,
    pub inlet_velocity: [Float; 3],
    pub density: Float,
    pub viscosity: Option<Float>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationConfig {
    pub max_iterations: usize,
    pub convergence_tolerance: Float,
    pub tau: Option<Float>, // relaxation time
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputConfig {
    pub output_directory: String,
    pub output_frequency: usize,
    pub output_format: String, // "vtk" or "vtu"
}

impl Config {
    pub fn from_file(path: &str) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Config = serde_json::from_str(&content)?;
        Ok(config)
    }

    pub fn calculate_tau(&self) -> Float {
        if let Some(tau) = self.simulation.tau {
            tau
        } else {
            // Calculate tau from Reynolds number and domain characteristics
            let characteristic_length = self.domain.dx;
            let characteristic_velocity = self.physics.inlet_velocity[0].max(
                self.physics.inlet_velocity[1].max(self.physics.inlet_velocity[2])
            );
            
            let viscosity = if let Some(nu) = self.physics.viscosity {
                nu
            } else {
                (characteristic_velocity * characteristic_length) / self.physics.reynolds_number
            };
            
            let cs2 = 1.0 / 3.0; // Speed of sound squared in lattice units
            3.0 * viscosity / cs2 + 0.5
        }
    }
}
