use anyhow::Result;
use std::fs::File;
use std::io::Write;
use crate::{config::Config, lattice::LatticePoint, Float};

pub struct VTKWriter {
    config: Config,
    collection_entries: Vec<(usize, f64, String)>, // (iteration, time, filename)
}

impl VTKWriter {
    pub fn new(config: &Config) -> Self {
        Self {
            config: config.clone(),
            collection_entries: Vec::new(),
        }
    }
    
    pub fn write(&mut self, filename: &str, lattice: &[LatticePoint], iteration: usize) -> Result<()> {
        let nx = self.config.domain.nx;
        let ny = self.config.domain.ny;
        let nz = self.config.domain.nz;
        
        let mut file = File::create(filename)?;
        
        // Calculate physical time (assuming unit time step for now)
        let time = iteration as f64;
        
        // Track this file for the collection
        self.collection_entries.push((iteration, time, filename.to_string()));
        
        // Write VTK header for structured grid
        writeln!(file, "# vtk DataFile Version 3.0")?;
        writeln!(file, "LBM Solution - Iteration {} Time {:.3}", iteration, time)?;
        writeln!(file, "ASCII")?;
        writeln!(file, "DATASET STRUCTURED_GRID")?;
        writeln!(file, "DIMENSIONS {} {} {}", nx, ny, nz)?;
        
        // Write points
        writeln!(file, "POINTS {} float", nx * ny * nz)?;
        for k in 0..nz {
            for j in 0..ny {
                for i in 0..nx {
                    let x = i as Float * self.config.domain.dx;
                    let y = j as Float * self.config.domain.dy;
                    let z = k as Float * self.config.domain.dz;
                    writeln!(file, "{} {} {}", x, y, z)?;
                }
            }
        }
        
        // Write point data
        writeln!(file, "POINT_DATA {}", nx * ny * nz)?;
        
        // Density
        writeln!(file, "SCALARS Density float")?;
        writeln!(file, "LOOKUP_TABLE default")?;
        for point in lattice {
            writeln!(file, "{:.6}", point.density)?;
        }
        
        // Velocity
        writeln!(file, "VECTORS Velocity float")?;
        for point in lattice {
            writeln!(file, "{:.6} {:.6} {:.6}", point.velocity[0], point.velocity[1], point.velocity[2])?;
        }
        
        // Velocity magnitude
        writeln!(file, "SCALARS VelocityMagnitude float")?;
        writeln!(file, "LOOKUP_TABLE default")?;
        for point in lattice {
            let vel_mag = (point.velocity[0].powi(2) + 
                          point.velocity[1].powi(2) + 
                          point.velocity[2].powi(2)).sqrt();
            writeln!(file, "{:.6}", vel_mag)?;
        }
        
        // Node type
        writeln!(file, "SCALARS NodeType float")?;
        writeln!(file, "LOOKUP_TABLE default")?;
        for point in lattice {
            writeln!(file, "{:.1}", point.node_type as f32)?;
        }
        
        // Geometry indicator (useful for visualizing solid regions)
        writeln!(file, "SCALARS GeometryType float")?;
        writeln!(file, "LOOKUP_TABLE default")?;
        for point in lattice {
            let value = match point.node_type {
                0 => 0.0,   // Fluid - blue
                1 => 1.0,   // Solid - red  
                2 => 0.5,   // Inlet - green
                3 => 0.25,  // Outlet - yellow
                _ => -1.0,  // Unknown - black
            };
            writeln!(file, "{:.2}", value)?;
        }
        
        // Pressure (from density)
        writeln!(file, "SCALARS Pressure float")?;
        writeln!(file, "LOOKUP_TABLE default")?;
        for point in lattice {
            let pressure = (point.density - self.config.physics.density) / 3.0; // cs^2 = 1/3
            writeln!(file, "{:.6}", pressure)?;
        }
        
        // Vorticity
        let vorticity = self.calculate_vorticity(lattice);
        writeln!(file, "VECTORS Vorticity float")?;
        for k in 0..nz {
            for j in 0..ny {
                for i in 0..nx {
                    let idx = i + j * nx + k * nx * ny;
                    writeln!(file, "{:.6} {:.6} {:.6}", 
                            vorticity[idx * 3], 
                            vorticity[idx * 3 + 1], 
                            vorticity[idx * 3 + 2])?;
                }
            }
        }
        
        Ok(())
    }
    
    fn calculate_vorticity(&self, lattice: &[LatticePoint]) -> Vec<Float> {
        let nx = self.config.domain.nx;
        let ny = self.config.domain.ny;
        let nz = self.config.domain.nz;
        let dx = self.config.domain.dx;
        let dy = self.config.domain.dy;
        let dz = self.config.domain.dz;
        
        let mut vorticity = vec![0.0; lattice.len() * 3];
        
        for k in 1..nz-1 {
            for j in 1..ny-1 {
                for i in 1..nx-1 {
                    let idx = i + j * nx + k * nx * ny;
                    
                    // Calculate velocity gradients using central differences
                    let idx_xp = (i + 1) + j * nx + k * nx * ny;
                    let idx_xm = (i - 1) + j * nx + k * nx * ny;
                    let idx_yp = i + (j + 1) * nx + k * nx * ny;
                    let idx_ym = i + (j - 1) * nx + k * nx * ny;
                    let idx_zp = i + j * nx + (k + 1) * nx * ny;
                    let idx_zm = i + j * nx + (k - 1) * nx * ny;
                    
                    // Only calculate for fluid nodes
                    if lattice[idx].node_type == 0 {
                        // dw/dy - dv/dz (x-component of vorticity)
                        let dwdy = (lattice[idx_yp].velocity[2] - lattice[idx_ym].velocity[2]) / (2.0 * dy);
                        let dvdz = (lattice[idx_zp].velocity[1] - lattice[idx_zm].velocity[1]) / (2.0 * dz);
                        vorticity[idx * 3] = dwdy - dvdz;
                        
                        // du/dz - dw/dx (y-component of vorticity)
                        let dudz = (lattice[idx_zp].velocity[0] - lattice[idx_zm].velocity[0]) / (2.0 * dz);
                        let dwdx = (lattice[idx_xp].velocity[2] - lattice[idx_xm].velocity[2]) / (2.0 * dx);
                        vorticity[idx * 3 + 1] = dudz - dwdx;
                        
                        // dv/dx - du/dy (z-component of vorticity)
                        let dvdx = (lattice[idx_xp].velocity[1] - lattice[idx_xm].velocity[1]) / (2.0 * dx);
                        let dudy = (lattice[idx_yp].velocity[0] - lattice[idx_ym].velocity[0]) / (2.0 * dy);
                        vorticity[idx * 3 + 2] = dvdx - dudy;
                    }
                }
            }
        }
        
        vorticity
    }
    
    pub fn write_geometry(&self, filename: &str, geometry: &crate::geometry::Geometry) -> Result<()> {
        let nx = self.config.domain.nx;
        let ny = self.config.domain.ny;
        let nz = self.config.domain.nz;
        
        let mut file = File::create(filename)?;
        
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
                    let x = i as Float * self.config.domain.dx;
                    let y = j as Float * self.config.domain.dy;
                    let z = k as Float * self.config.domain.dz;
                    writeln!(file, "{} {} {}", x, y, z)?;
                }
            }
        }
        
        // Write point data
        writeln!(file, "POINT_DATA {}", nx * ny * nz)?;
        
        // Node type
        writeln!(file, "SCALARS NodeType float")?;
        writeln!(file, "LOOKUP_TABLE default")?;
        for k in 0..nz {
            for j in 0..ny {
                for i in 0..nx {
                    let node_type = if geometry.is_solid(i, j, k) {
                        1.0
                    } else if geometry.is_inlet(i, j, k) {
                        2.0
                    } else if geometry.is_outlet(i, j, k) {
                        3.0
                    } else {
                        0.0
                    };
                    writeln!(file, "{:.1}", node_type)?;
                }
            }
        }
        
        Ok(())
    }
    
    /// Write a ParaView collection file that groups all VTK files with time information
    pub fn write_collection(&self, collection_filename: &str) -> Result<()> {
        let mut file = File::create(collection_filename)?;
        
        writeln!(file, "<?xml version=\"1.0\"?>")?;
        writeln!(file, "<VTKFile type=\"Collection\" version=\"0.1\">")?;
        writeln!(file, "  <Collection>")?;
        
        for (_iteration, time, filename) in &self.collection_entries {
            // Extract just the filename (not the full path) for the collection
            let basename = std::path::Path::new(filename)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(filename);
            writeln!(file, "    <DataSet timestep=\"{:.6}\" part=\"0\" file=\"{}\"/>", 
                     time, basename)?;
        }
        
        writeln!(file, "  </Collection>")?;
        writeln!(file, "</VTKFile>")?;
        
        Ok(())
    }
    
    /// Get the number of files written so far
    pub fn get_file_count(&self) -> usize {
        self.collection_entries.len()
    }
}
