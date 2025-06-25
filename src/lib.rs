pub mod config;
pub mod geometry;
pub mod lattice;
pub mod solver;
pub mod gpu;
pub mod output;

pub use config::Config;
pub use geometry::Geometry;
pub use lattice::{D3Q27, LatticePoint};
pub use solver::LBMSolver;
pub use gpu::GPUContext;
pub use output::VTKWriter;

pub type Float = f32;
