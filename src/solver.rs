use anyhow::Result;
use log::info;
use crate::{
    config::Config,
    geometry::Geometry,
    lattice::LatticePoint,
    gpu::GPUContext,
    output::VTKWriter,
};

pub struct LBMSolver {
    config: Config,
    geometry: Geometry,
    gpu_context: GPUContext,
    lattice: Vec<LatticePoint>,
    iteration: usize,
}

impl LBMSolver {
    pub async fn new(config: Config, stl_path: &str) -> Result<Self> {
        info!("Initializing LBM solver with domain size: {}x{}x{}", 
              config.domain.nx, config.domain.ny, config.domain.nz);
        
        // Load geometry from STL file
        let geometry = Geometry::from_stl(stl_path, &config.domain)?;
        info!("Loaded geometry with {} solid nodes, {} fluid nodes", 
              geometry.solid_nodes.len(), geometry.fluid_nodes.len());
        
        // Initialize GPU context
        let gpu_context = GPUContext::new(&config).await?;
        
        // Initialize lattice
        let total_nodes = config.domain.nx * config.domain.ny * config.domain.nz;
        let mut lattice = Vec::with_capacity(total_nodes);
        
        for k in 0..config.domain.nz {
            for j in 0..config.domain.ny {
                for i in 0..config.domain.nx {
                    let node_type = if geometry.is_solid(i, j, k) {
                        1 // Solid
                    } else if geometry.is_inlet(i, j, k) {
                        2 // Inlet
                    } else if geometry.is_outlet(i, j, k) {
                        3 // Outlet
                    } else {
                        0 // Fluid
                    };
                    
                    let velocity = if node_type == 2 {
                        config.physics.inlet_velocity
                    } else {
                        [0.0; 3]
                    };
                    
                    let point = LatticePoint::new_equilibrium(
                        config.physics.density,
                        velocity,
                        node_type,
                    );
                    
                    lattice.push(point);
                }
            }
        }
        
        // Upload initial data to GPU
        gpu_context.upload_lattice_data(&lattice);
        
        Ok(Self {
            config,
            geometry,
            gpu_context,
            lattice,
            iteration: 0,
        })
    }
    
    pub async fn run(&mut self) -> Result<()> {
        info!("Starting LBM simulation for {} iterations", self.config.simulation.max_iterations);
        
        // Create output directory
        std::fs::create_dir_all(&self.config.output.output_directory)?;
        
        // Write initial state
        if self.iteration % self.config.output.output_frequency == 0 {
            self.write_output().await?;
        }
        
        let mut converged = false;
        
        while self.iteration < self.config.simulation.max_iterations && !converged {
            // Perform one LBM step on GPU
            self.gpu_context.step();
            
            self.iteration += 1;
            
            // Output results at specified frequency
            if self.iteration % self.config.output.output_frequency == 0 {
                self.write_output().await?;
                
                // Check convergence (simplified)
                converged = self.check_convergence().await?;
                
                info!("Iteration {}: {}", self.iteration, 
                      if converged { "Converged" } else { "Continuing" });
            }
        }
        
        if converged {
            info!("Simulation converged after {} iterations", self.iteration);
        } else {
            info!("Simulation completed {} iterations", self.iteration);
        }
        
        Ok(())
    }
    
    async fn write_output(&mut self) -> Result<()> {
        // Read data back from GPU
        self.lattice = self.gpu_context.read_lattice_data().await?;
        
        // Write VTK file
        let filename = format!("{}/output_{:06}.{}", 
                              self.config.output.output_directory,
                              self.iteration,
                              self.config.output.output_format);
        
        let vtk_writer = VTKWriter::new(&self.config);
        vtk_writer.write(&filename, &self.lattice, self.iteration)?;
        
        info!("Wrote output file: {}", filename);
        
        Ok(())
    }
    
    async fn check_convergence(&self) -> Result<bool> {
        // Simple convergence check based on maximum velocity change
        // In a real implementation, you would compare with previous iteration
        
        let max_velocity = self.lattice.iter()
            .filter(|point| point.node_type == 0) // Only fluid nodes
            .map(|point| {
                let v_mag = (point.velocity[0].powi(2) + 
                           point.velocity[1].powi(2) + 
                           point.velocity[2].powi(2)).sqrt();
                v_mag
            })
            .fold(0.0, f32::max);
        
        // Simple convergence criterion
        Ok(max_velocity < self.config.simulation.convergence_tolerance)
    }
    
    pub fn get_iteration(&self) -> usize {
        self.iteration
    }
    
    pub fn get_config(&self) -> &Config {
        &self.config
    }
    
    pub fn get_geometry(&self) -> &Geometry {
        &self.geometry
    }
}
