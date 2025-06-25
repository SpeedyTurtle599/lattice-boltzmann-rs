use nalgebra::Point3;
use stl_io::read_stl;
use std::collections::HashSet;
use crate::config::DomainConfig;

#[derive(Debug, Clone)]
pub struct Geometry {
    pub solid_nodes: HashSet<(usize, usize, usize)>,
    pub boundary_nodes: HashSet<(usize, usize, usize)>,
    pub fluid_nodes: HashSet<(usize, usize, usize)>,
    pub inlet_nodes: HashSet<(usize, usize, usize)>,
    pub outlet_nodes: HashSet<(usize, usize, usize)>,
}

impl Geometry {
    pub fn from_stl(stl_path: &str, domain: &DomainConfig) -> anyhow::Result<Self> {
        let mut file = std::fs::File::open(stl_path)?;
        let stl = read_stl(&mut file)?;
        
        let mut solid_nodes = HashSet::new();
        let mut boundary_nodes = HashSet::new();
        
        // Convert STL mesh to voxelized geometry
        for face in stl.faces {
            let vertices = [
                Point3::new(stl.vertices[face.vertices[0]][0], stl.vertices[face.vertices[0]][1], stl.vertices[face.vertices[0]][2]),
                Point3::new(stl.vertices[face.vertices[1]][0], stl.vertices[face.vertices[1]][1], stl.vertices[face.vertices[1]][2]),
                Point3::new(stl.vertices[face.vertices[2]][0], stl.vertices[face.vertices[2]][1], stl.vertices[face.vertices[2]][2]),
            ];
            
            // Voxelize triangle using scanline algorithm
            Self::voxelize_triangle(&vertices, domain, &mut solid_nodes, &mut boundary_nodes);
        }
        
        // Generate fluid nodes (all nodes not solid)
        let mut fluid_nodes = HashSet::new();
        for i in 0..domain.nx {
            for j in 0..domain.ny {
                for k in 0..domain.nz {
                    if !solid_nodes.contains(&(i, j, k)) {
                        fluid_nodes.insert((i, j, k));
                    }
                }
            }
        }
        
        // Define inlet and outlet based on domain boundaries
        let mut inlet_nodes = HashSet::new();
        let mut outlet_nodes = HashSet::new();
        
        // Inlet at x=0 plane
        for j in 0..domain.ny {
            for k in 0..domain.nz {
                if fluid_nodes.contains(&(0, j, k)) {
                    inlet_nodes.insert((0, j, k));
                }
            }
        }
        
        // Outlet at x=nx-1 plane
        for j in 0..domain.ny {
            for k in 0..domain.nz {
                if fluid_nodes.contains(&(domain.nx - 1, j, k)) {
                    outlet_nodes.insert((domain.nx - 1, j, k));
                }
            }
        }
        
        Ok(Geometry {
            solid_nodes,
            boundary_nodes,
            fluid_nodes,
            inlet_nodes,
            outlet_nodes,
        })
    }
    
    fn voxelize_triangle(
        vertices: &[Point3<f32>; 3],
        domain: &DomainConfig,
        solid_nodes: &mut HashSet<(usize, usize, usize)>,
        boundary_nodes: &mut HashSet<(usize, usize, usize)>,
    ) {
        // Simple voxelization: mark nodes within triangle bounding box
        let min_x = vertices.iter().map(|v| v.x).fold(f32::INFINITY, f32::min);
        let max_x = vertices.iter().map(|v| v.x).fold(f32::NEG_INFINITY, f32::max);
        let min_y = vertices.iter().map(|v| v.y).fold(f32::INFINITY, f32::min);
        let max_y = vertices.iter().map(|v| v.y).fold(f32::NEG_INFINITY, f32::max);
        let min_z = vertices.iter().map(|v| v.z).fold(f32::INFINITY, f32::min);
        let max_z = vertices.iter().map(|v| v.z).fold(f32::NEG_INFINITY, f32::max);
        
        let i_min = ((min_x / domain.dx) as usize).max(0).min(domain.nx - 1);
        let i_max = ((max_x / domain.dx) as usize).max(0).min(domain.nx - 1);
        let j_min = ((min_y / domain.dy) as usize).max(0).min(domain.ny - 1);
        let j_max = ((max_y / domain.dy) as usize).max(0).min(domain.ny - 1);
        let k_min = ((min_z / domain.dz) as usize).max(0).min(domain.nz - 1);
        let k_max = ((max_z / domain.dz) as usize).max(0).min(domain.nz - 1);
        
        for i in i_min..=i_max {
            for j in j_min..=j_max {
                for k in k_min..=k_max {
                    let point = Point3::new(
                        i as f32 * domain.dx,
                        j as f32 * domain.dy,
                        k as f32 * domain.dz,
                    );
                    
                    if Self::point_in_triangle(&point, vertices) {
                        solid_nodes.insert((i, j, k));
                        
                        // Mark neighboring nodes as boundary
                        for di in -1i32..=1 {
                            for dj in -1i32..=1 {
                                for dk in -1i32..=1 {
                                    if di == 0 && dj == 0 && dk == 0 { continue; }
                                    
                                    let ni = (i as i32 + di) as usize;
                                    let nj = (j as i32 + dj) as usize;
                                    let nk = (k as i32 + dk) as usize;
                                    
                                    if ni < domain.nx && nj < domain.ny && nk < domain.nz {
                                        boundary_nodes.insert((ni, nj, nk));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    
    fn point_in_triangle(point: &Point3<f32>, triangle: &[Point3<f32>; 3]) -> bool {
        // Simplified point-in-triangle test using barycentric coordinates
        let v0 = triangle[2] - triangle[0];
        let v1 = triangle[1] - triangle[0];
        let v2 = point - triangle[0];
        
        let dot00 = v0.dot(&v0);
        let dot01 = v0.dot(&v1);
        let dot02 = v0.dot(&v2);
        let dot11 = v1.dot(&v1);
        let dot12 = v1.dot(&v2);
        
        let inv_denom = 1.0 / (dot00 * dot11 - dot01 * dot01);
        let u = (dot11 * dot02 - dot01 * dot12) * inv_denom;
        let v = (dot00 * dot12 - dot01 * dot02) * inv_denom;
        
        (u >= 0.0) && (v >= 0.0) && (u + v <= 1.0)
    }
    
    pub fn is_solid(&self, i: usize, j: usize, k: usize) -> bool {
        self.solid_nodes.contains(&(i, j, k))
    }
    
    pub fn is_boundary(&self, i: usize, j: usize, k: usize) -> bool {
        self.boundary_nodes.contains(&(i, j, k))
    }
    
    pub fn is_fluid(&self, i: usize, j: usize, k: usize) -> bool {
        self.fluid_nodes.contains(&(i, j, k))
    }
    
    pub fn is_inlet(&self, i: usize, j: usize, k: usize) -> bool {
        self.inlet_nodes.contains(&(i, j, k))
    }
    
    pub fn is_outlet(&self, i: usize, j: usize, k: usize) -> bool {
        self.outlet_nodes.contains(&(i, j, k))
    }
}
