use anyhow::Result;
use log::info;
use indicatif::{ProgressBar, ProgressStyle};
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
    vtk_writer: VTKWriter,
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
                    
                    // Initialize velocity based on node type
                    let velocity = if node_type == 2 {
                        // Inlet velocity
                        config.physics.inlet_velocity
                    } else if node_type == 0 {
                        // Fluid nodes start with small initial velocity to seed the flow
                        [config.physics.inlet_velocity[0] * 0.1, 0.0, 0.0]
                    } else {
                        // Solid and outlet nodes
                        [0.0; 3]
                    };
                    
                    let point = LatticePoint::new_equilibrium(
                        config.physics.density,
                        velocity,
                        node_type,
                    );
                    
                    lattice.push(point);
                    
                    // Debug output for some key nodes (use debug level to avoid interfering with progress bar)
                    if (i == 0 && j == config.domain.ny / 2 && k == config.domain.nz / 2) ||
                       (i == config.domain.nx / 2 && j == config.domain.ny / 2 && k == config.domain.nz / 2) {
                        log::debug!("Node ({}, {}, {}): type={}, vel=[{:.4}, {:.4}, {:.4}]", 
                                i, j, k, node_type, velocity[0], velocity[1], velocity[2]);
                    }
                }
            }
        }
        
        // Upload initial data to GPU
        gpu_context.upload_lattice_data(&lattice);
        
        // Write geometry file for visualization debugging
        Self::write_geometry_file(&geometry, &config)?;
        
        // Initialize VTK writer
        let vtk_writer = VTKWriter::new(&config);
        
        Ok(Self {
            config,
            geometry,
            gpu_context,
            lattice,
            iteration: 0,
            vtk_writer,
        })
    }
    
    pub async fn run(&mut self) -> Result<()> {
        info!("Starting LBM simulation for {} iterations", self.config.simulation.max_iterations);
        
        // Create output directory
        std::fs::create_dir_all(&self.config.output.output_directory)?;
        
        // Create progress bar with cleaner format
        let pb = ProgressBar::new(self.config.simulation.max_iterations as u64);
        pb.set_style(ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {pos}/{len} ({eta}) {msg}")
            .unwrap()
            .progress_chars("#>-"));
        pb.set_message("LBM Simulation");
        
        // Write initial state
        if self.iteration % self.config.output.output_frequency == 0 {
            self.write_output().await?;
        }
        
        let mut converged = false;
        
        while self.iteration < self.config.simulation.max_iterations && !converged {
            // Perform one LBM step on GPU
            self.gpu_context.step()?;
            
            self.iteration += 1;
            pb.set_position(self.iteration as u64);
            
            // Output results at specified frequency
            if self.iteration % self.config.output.output_frequency == 0 {
                pb.set_message("Writing...");
                self.write_output().await?;
                
                // Check convergence (simplified)
                converged = self.check_convergence().await?;
                
                if converged {
                    pb.set_message("Converged!");
                } else {
                    pb.set_message("LBM Simulation");
                }
            }
        }
        
        pb.finish_with_message(format!("LBM Simulation completed - {} iterations", self.iteration));
        
        // Write ParaView collection file for time series
        let collection_filename = format!("{}/simulation.pvd", self.config.output.output_directory);
        self.vtk_writer.write_collection(&collection_filename)?;
        info!("Wrote ParaView collection file: {}", collection_filename);
        info!("To view time evolution in ParaView, open the .pvd file instead of individual .vtk files");
        
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
        
        // Calculate flow statistics for diagnostics
        let mut max_velocity = 0.0;
        let mut avg_velocity = 0.0;
        let mut fluid_count = 0;
        let mut inlet_velocity_check = 0.0;
        let mut inlet_count = 0;
        
        for point in &self.lattice {
            if point.node_type == 0 { // Fluid nodes
                let v_mag = (point.velocity[0].powi(2) + 
                           point.velocity[1].powi(2) + 
                           point.velocity[2].powi(2)).sqrt();
                max_velocity = f32::max(max_velocity, v_mag);
                avg_velocity += v_mag;
                fluid_count += 1;
            } else if point.node_type == 2 { // Inlet nodes
                let v_mag = (point.velocity[0].powi(2) + 
                           point.velocity[1].powi(2) + 
                           point.velocity[2].powi(2)).sqrt();
                inlet_velocity_check += v_mag;
                inlet_count += 1;
            }
        }
        
        if fluid_count > 0 {
            avg_velocity /= fluid_count as f32;
        }
        if inlet_count > 0 {
            inlet_velocity_check /= inlet_count as f32;
        }
        
        log::debug!("Iteration {}: max_vel={:.6}, avg_vel={:.6}, inlet_vel={:.6} ({} inlet nodes)", 
              self.iteration, max_velocity, avg_velocity, inlet_velocity_check, inlet_count);
        
        // Write VTK file
        let filename = format!("{}/output_{:06}.{}", 
                              self.config.output.output_directory,
                              self.iteration,
                              self.config.output.output_format);
        
        self.vtk_writer.write(&filename, &self.lattice, self.iteration)?;
        
        log::debug!("Wrote output file: {}", filename);
        
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
    
    fn write_geometry_file(geometry: &Geometry, config: &Config) -> Result<()> {
        use std::fs::File;
        use std::io::Write;
        
        let filename = format!("{}/geometry.vtk", config.output.output_directory);
        let mut file = File::create(&filename)?;
        
        let nx = config.domain.nx;
        let ny = config.domain.ny;
        let nz = config.domain.nz;
        
        // Write VTK header for structured grid
        writeln!(file, "# vtk DataFile Version 3.0")?;
        writeln!(file, "LBM Geometry")?;
        writeln!(file, "ASCII")?;
        writeln!(file, "DATASET STRUCTURED_GRID")?;
        writeln!(file, "DIMENSIONS {} {} {}", nx, ny, nz)?;
        
        // Write points
        writeln!(file, "POINTS {} float", nx * ny * nz)?;
        for k in 0..nz {
            for j in 0..ny {
                for i in 0..nx {
                    let x = i as f32 * config.domain.dx;
                    let y = j as f32 * config.domain.dy;
                    let z = k as f32 * config.domain.dz;
                    writeln!(file, "{} {} {}", x, y, z)?;
                }
            }
        }
        
        // Write point data
        writeln!(file, "POINT_DATA {}", nx * ny * nz)?;
        
        // Node classification
        writeln!(file, "SCALARS NodeClassification float 1")?;
        writeln!(file, "LOOKUP_TABLE default")?;
        for k in 0..nz {
            for j in 0..ny {
                for i in 0..nx {
                    let value = if geometry.is_solid(i, j, k) {
                        1.0 // Solid
                    } else if geometry.is_inlet(i, j, k) {
                        0.5 // Inlet
                    } else if geometry.is_outlet(i, j, k) {
                        0.25 // Outlet
                    } else {
                        0.0 // Fluid
                    };
                    writeln!(file, "{}", value)?;
                }
            }
        }
        
        info!("Wrote geometry file: {}", filename);
        Ok(())
    }
}
