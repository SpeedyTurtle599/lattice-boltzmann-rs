use lattice_boltzmann_rs::{Config, LBMSolver, VTKWriter};
use anyhow::Result;
use env_logger;
use log::info;
use std::env;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    env_logger::init();
    
    // Parse command line arguments
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: {} <config.json> <geometry.stl>", args[0]);
        eprintln!("  config.json - JSON file containing simulation parameters");
        eprintln!("  geometry.stl - STL file containing the geometry");
        std::process::exit(1);
    }
    
    let config_path = &args[1];
    let stl_path = &args[2];
    
    info!("Loading configuration from: {}", config_path);
    let config = Config::from_file(config_path)?;
    
    info!("Simulation parameters:");
    info!("  Domain: {}x{}x{}", config.domain.nx, config.domain.ny, config.domain.nz);
    info!("  Reynolds number: {}", config.physics.reynolds_number);
    info!("  Inlet velocity: {:?}", config.physics.inlet_velocity);
    info!("  Max iterations: {}", config.simulation.max_iterations);
    info!("  Output frequency: {}", config.output.output_frequency);
    info!("  Tau (relaxation time): {}", config.calculate_tau());
    
    // Create and run solver
    info!("Initializing LBM solver...");
    let mut solver = LBMSolver::new(config, stl_path).await?;
    
    // Write geometry file for visualization
    let vtk_writer = VTKWriter::new(solver.get_config());
    let geometry_filename = format!("{}/geometry.vtk", solver.get_config().output.output_directory);
    vtk_writer.write_geometry(&geometry_filename, solver.get_geometry())?;
    info!("Wrote geometry file: {}", geometry_filename);
    
    // Run simulation
    info!("Starting simulation...");
    solver.run().await?;
    
    info!("Simulation completed successfully!");
    info!("Output files written to: {}", solver.get_config().output.output_directory);
    info!("To visualize:");
    info!("  1. Open ParaView");
    info!("  2. Load the geometry.vtk file to see the domain setup");
    info!("  3. Load output_*.vtk files to see the transient solution");
    info!("  4. Use the 'Velocity' vector field for streamlines");
    info!("  5. Use 'VelocityMagnitude' or 'Pressure' for contour plots");
    info!("  6. Use 'Vorticity' to visualize flow structures");
    
    Ok(())
}
