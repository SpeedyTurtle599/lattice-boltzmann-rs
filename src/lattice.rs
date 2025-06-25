use bytemuck::{Pod, Zeroable};
use crate::Float;

/// D3Q27 Lattice-Boltzmann model constants and structures
pub struct D3Q27;

impl D3Q27 {
    /// Number of discrete velocities
    pub const Q: usize = 27;
    
    /// Discrete velocities (27 directions in 3D)
    pub const VELOCITIES: [[i32; 3]; 27] = [
        // Center
        [0, 0, 0],
        // Face neighbors (6)
        [1, 0, 0], [-1, 0, 0], [0, 1, 0], [0, -1, 0], [0, 0, 1], [0, 0, -1],
        // Edge neighbors (12)
        [1, 1, 0], [1, -1, 0], [-1, 1, 0], [-1, -1, 0],
        [1, 0, 1], [1, 0, -1], [-1, 0, 1], [-1, 0, -1],
        [0, 1, 1], [0, 1, -1], [0, -1, 1], [0, -1, -1],
        // Corner neighbors (8)
        [1, 1, 1], [1, 1, -1], [1, -1, 1], [1, -1, -1],
        [-1, 1, 1], [-1, 1, -1], [-1, -1, 1], [-1, -1, -1],
    ];
    
    /// Weights for each direction
    pub const WEIGHTS: [Float; 27] = [
        // Center
        8.0/27.0,
        // Face neighbors (6)
        2.0/27.0, 2.0/27.0, 2.0/27.0, 2.0/27.0, 2.0/27.0, 2.0/27.0,
        // Edge neighbors (12)
        1.0/54.0, 1.0/54.0, 1.0/54.0, 1.0/54.0,
        1.0/54.0, 1.0/54.0, 1.0/54.0, 1.0/54.0,
        1.0/54.0, 1.0/54.0, 1.0/54.0, 1.0/54.0,
        // Corner neighbors (8)
        1.0/216.0, 1.0/216.0, 1.0/216.0, 1.0/216.0,
        1.0/216.0, 1.0/216.0, 1.0/216.0, 1.0/216.0,
    ];
    
    /// Opposite directions for bounce-back boundary conditions
    pub const OPPOSITE: [usize; 27] = [
        0,  // Center stays the same
        2, 1, 4, 3, 6, 5,  // Face opposites
        9, 8, 7, 10, 13, 12, 11, 14, 17, 16, 15, 18,  // Edge opposites
        26, 25, 24, 23, 22, 21, 20, 19,  // Corner opposites
    ];
    
    /// Speed of sound squared
    pub const CS2: Float = 1.0 / 3.0;
}

/// A single lattice point containing distribution functions
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct LatticePoint {
    /// Distribution functions (f_i)
    pub f: [Float; 27],
    /// Macroscopic density
    pub density: Float,
    /// Macroscopic velocity
    pub velocity: [Float; 3],
    /// Node type (0: fluid, 1: solid, 2: inlet, 3: outlet)
    pub node_type: u32,
    /// Padding for alignment
    pub _padding: [u32; 3],
}

impl Default for LatticePoint {
    fn default() -> Self {
        Self {
            f: [0.0; 27],
            density: 1.0,
            velocity: [0.0; 3],
            node_type: 0,
            _padding: [0; 3],
        }
    }
}

impl LatticePoint {
    /// Initialize with equilibrium distribution
    pub fn new_equilibrium(density: Float, velocity: [Float; 3], node_type: u32) -> Self {
        let mut point = Self {
            f: [0.0; 27],
            density,
            velocity,
            node_type,
            _padding: [0; 3],
        };
        
        // Calculate equilibrium distribution
        for i in 0..D3Q27::Q {
            point.f[i] = Self::equilibrium_distribution(i, density, velocity);
        }
        
        point
    }
    
    /// Calculate equilibrium distribution function
    pub fn equilibrium_distribution(direction: usize, density: Float, velocity: [Float; 3]) -> Float {
        let weight = D3Q27::WEIGHTS[direction];
        let c = D3Q27::VELOCITIES[direction];
        
        // Dot product of velocity and lattice velocity
        let cu = c[0] as Float * velocity[0] + c[1] as Float * velocity[1] + c[2] as Float * velocity[2];
        
        // Velocity magnitude squared
        let u2 = velocity[0] * velocity[0] + velocity[1] * velocity[1] + velocity[2] * velocity[2];
        
        // Equilibrium distribution function
        weight * density * (1.0 + cu / D3Q27::CS2 + 
                           cu * cu / (2.0 * D3Q27::CS2 * D3Q27::CS2) - 
                           u2 / (2.0 * D3Q27::CS2))
    }
    
    /// Calculate macroscopic quantities from distribution functions
    pub fn calculate_macroscopic(&mut self) {
        // Density
        self.density = self.f.iter().sum();
        
        // Velocity
        self.velocity = [0.0; 3];
        for i in 0..D3Q27::Q {
            let c = D3Q27::VELOCITIES[i];
            self.velocity[0] += self.f[i] * c[0] as Float;
            self.velocity[1] += self.f[i] * c[1] as Float;
            self.velocity[2] += self.f[i] * c[2] as Float;
        }
        
        if self.density > 1e-10 {
            self.velocity[0] /= self.density;
            self.velocity[1] /= self.density;
            self.velocity[2] /= self.density;
        }
    }
    
    /// Apply BGK collision step
    pub fn collide(&mut self, tau: Float) {
        let omega = 1.0 / tau;
        
        for i in 0..D3Q27::Q {
            let f_eq = Self::equilibrium_distribution(i, self.density, self.velocity);
            self.f[i] += omega * (f_eq - self.f[i]);
        }
    }
}
