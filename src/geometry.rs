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
        
        // Inlet at x=0 plane - force ALL these to be inlet nodes
        for j in 0..domain.ny {
            for k in 0..domain.nz {
                let node = (0, j, k);
                // Remove from solid nodes if accidentally marked
                solid_nodes.remove(&node);
                // Ensure it's in fluid nodes
                fluid_nodes.insert(node);
                // Add to inlet
                inlet_nodes.insert(node);
                
                // Log inlet node assignment for debugging
                if j == domain.ny / 2 && k == domain.nz / 2 {
                    println!("Assigned inlet node at center: ({}, {}, {})", 0, j, k);
                }
            }
        }
        
        // Outlet at x=nx-1 plane - force ALL these to be outlet nodes
        for j in 0..domain.ny {
            for k in 0..domain.nz {
                let node = (domain.nx - 1, j, k);
                // Remove from solid nodes if accidentally marked
                solid_nodes.remove(&node);
                // Ensure it's in fluid nodes
                fluid_nodes.insert(node);
                // Add to outlet
                outlet_nodes.insert(node);
            }
        }
        
        // Log geometry statistics
        println!("Geometry loaded: {} solid, {} fluid, {} inlet, {} outlet nodes", 
                solid_nodes.len(), fluid_nodes.len(), inlet_nodes.len(), outlet_nodes.len());
        
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
        // Get triangle bounding box
        let min_x = vertices.iter().map(|v| v.x).fold(f32::INFINITY, f32::min);
        let max_x = vertices.iter().map(|v| v.x).fold(f32::NEG_INFINITY, f32::max);
        let min_y = vertices.iter().map(|v| v.y).fold(f32::INFINITY, f32::min);
        let max_y = vertices.iter().map(|v| v.y).fold(f32::NEG_INFINITY, f32::max);
        let min_z = vertices.iter().map(|v| v.z).fold(f32::INFINITY, f32::min);
        let max_z = vertices.iter().map(|v| v.z).fold(f32::NEG_INFINITY, f32::max);
        
        // Convert to grid indices with safety bounds
        let i_min = ((min_x / domain.dx).floor() as i32).max(0).min(domain.nx as i32 - 1) as usize;
        let i_max = ((max_x / domain.dx).ceil() as i32).max(0).min(domain.nx as i32 - 1) as usize;
        let j_min = ((min_y / domain.dy).floor() as i32).max(0).min(domain.ny as i32 - 1) as usize;
        let j_max = ((max_y / domain.dy).ceil() as i32).max(0).min(domain.ny as i32 - 1) as usize;
        let k_min = ((min_z / domain.dz).floor() as i32).max(0).min(domain.nz as i32 - 1) as usize;
        let k_max = ((max_z / domain.dz).ceil() as i32).max(0).min(domain.nz as i32 - 1) as usize;
        
        // Use multiple sampling points per voxel for better accuracy
        let samples_per_axis = 3;
        let total_samples = samples_per_axis * samples_per_axis * samples_per_axis;
        
        // Sample each voxel in the bounding box
        for i in i_min..=i_max {
            for j in j_min..=j_max {
                for k in k_min..=k_max {
                    let mut inside_count = 0;
                    
                    // Multiple sampling points within each voxel
                    for si in 0..samples_per_axis {
                        for sj in 0..samples_per_axis {
                            for sk in 0..samples_per_axis {
                                let offset_x = (si as f32 + 0.5) / samples_per_axis as f32;
                                let offset_y = (sj as f32 + 0.5) / samples_per_axis as f32;
                                let offset_z = (sk as f32 + 0.5) / samples_per_axis as f32;
                                
                                let point = Point3::new(
                                    (i as f32 + offset_x) * domain.dx,
                                    (j as f32 + offset_y) * domain.dy,
                                    (k as f32 + offset_z) * domain.dz,
                                );
                                
                                // Check if point is inside the geometry using better method
                                if Self::point_inside_triangle_volume(&point, vertices, domain) {
                                    inside_count += 1;
                                }
                            }
                        }
                    }
                    
                    // Mark as solid if majority of sample points are inside
                    if inside_count > total_samples / 2 {
                        solid_nodes.insert((i, j, k));
                        // println!("Marked solid node at ({}, {}, {})", i, j, k);
                        
                        // Mark neighboring nodes as boundary candidates
                        for di in -1i32..=1 {
                            for dj in -1i32..=1 {
                                for dk in -1i32..=1 {
                                    if di == 0 && dj == 0 && dk == 0 { continue; }
                                    
                                    let ni = i as i32 + di;
                                    let nj = j as i32 + dj;
                                    let nk = k as i32 + dk;
                                    
                                    if ni >= 0 && ni < domain.nx as i32 && 
                                       nj >= 0 && nj < domain.ny as i32 && 
                                       nk >= 0 && nk < domain.nz as i32 {
                                        boundary_nodes.insert((ni as usize, nj as usize, nk as usize));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    
    fn point_inside_triangle_volume(point: &Point3<f32>, triangle: &[Point3<f32>; 3], domain: &DomainConfig) -> bool {
        // First check distance to triangle plane
        let distance = Self::point_triangle_distance(point, triangle);
        let thickness_threshold = (domain.dx.min(domain.dy).min(domain.dz)) * 0.8;
        
        // If the point is close to the triangle surface, consider it inside
        distance < thickness_threshold
    }
    
    fn point_triangle_distance(point: &Point3<f32>, triangle: &[Point3<f32>; 3]) -> f32 {
        // Compute vectors
        let v0 = triangle[1] - triangle[0];
        let v1 = triangle[2] - triangle[0];
        let v2 = point - triangle[0];
        
        // Compute dot products
        let dot00 = v0.dot(&v0);
        let dot01 = v0.dot(&v1);
        let dot02 = v0.dot(&v2);
        let dot11 = v1.dot(&v1);
        let dot12 = v1.dot(&v2);
        
        // Compute barycentric coordinates
        let inv_denom = 1.0 / (dot00 * dot11 - dot01 * dot01);
        let u = (dot11 * dot02 - dot01 * dot12) * inv_denom;
        let v = (dot00 * dot12 - dot01 * dot02) * inv_denom;
        
        if u >= 0.0 && v >= 0.0 && u + v <= 1.0 {
            // Point is inside triangle, compute distance to plane
            let normal = v0.cross(&v1).normalize();
            (v2.dot(&normal)).abs()
        } else {
            // Point is outside triangle, find distance to closest edge/vertex
            let d1 = Self::point_line_segment_distance(point, &triangle[0], &triangle[1]);
            let d2 = Self::point_line_segment_distance(point, &triangle[1], &triangle[2]);
            let d3 = Self::point_line_segment_distance(point, &triangle[2], &triangle[0]);
            d1.min(d2).min(d3)
        }
    }
    
    fn point_line_segment_distance(point: &Point3<f32>, a: &Point3<f32>, b: &Point3<f32>) -> f32 {
        let ab = b - a;
        let ap = point - a;
        let ab_len_sq = ab.dot(&ab);
        
        if ab_len_sq == 0.0 {
            return ap.magnitude();
        }
        
        let t = (ap.dot(&ab) / ab_len_sq).clamp(0.0, 1.0);
        let projection = a + ab * t;
        (point - projection).magnitude()
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
