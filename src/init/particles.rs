use nannou::prelude::*;

use system4::particle::ParticleSystem;

const DEFAULT_PARTICLE_SIZE: f32 = 4.0;
const DEFAULT_PARTICLE_RGB: (f32, f32, f32) = (0.73, 0.73, 0.74);

/// Initialize particle system
///
/// # Arguments
/// * `render_size` - Size of render area
/// * `particle_limit` - Maximum number of particles
/// * `use_gpu` - Whether to enable GPU particle simulation (Phase 4) or use CPU (legacy)
/// * `device` - WebGPU device (required if use_gpu is true)
///
/// # Returns
/// Initialized ParticleSystem
pub fn init_particle_system(
    render_size: Vec2,
    particle_limit: u32,
    use_gpu: bool,
    device: Option<&wgpu::Device>,
) -> ParticleSystem {
    let mut particle_system = ParticleSystem::new(
        pt2(0.0, 0.0),
        render_size.x,
        render_size.y,
        DEFAULT_PARTICLE_SIZE,
        rgb(
            DEFAULT_PARTICLE_RGB.0,
            DEFAULT_PARTICLE_RGB.1,
            DEFAULT_PARTICLE_RGB.2,
        ),
        particle_limit,
    );

    particle_system.set_mass_variation_enabled(true);
    particle_system.set_mass_variation_amount(0.5);

    // Initialize GPU components if requested
    if use_gpu {
        println!("Initializing GPU particle system...");
        println!("  Particle limit: {}", particle_limit);
        println!("  Render size: {}x{}", render_size.x, render_size.y);

        if let Some(dev) = device {
            // Initialize GPU force field
            println!("  Initializing GPU force field...");
            if let Err(e) = particle_system.init_gpu_force_field(dev, true) {
                eprintln!(
                    "Warning: Failed to initialize GPU force field: {}. Falling back to CPU forces.",
                    e
                );
                particle_system.set_use_gpu_forces(false);
            } else {
                println!("  GPU force field initialized successfully");
            }

            // Initialize GPU physics (Phase 4)
            println!("  Initializing GPU physics (Phase 4)...");
            if let Err(e) = particle_system.init_gpu_physics(dev, particle_limit as usize, true) {
                eprintln!(
                    "Warning: Failed to initialize GPU physics: {}. Falling back to CPU physics.",
                    e
                );
                particle_system.set_use_gpu_physics(false);
            } else {
                println!("  GPU physics initialized successfully");
                println!("GPU particle system ready!");
            }
        } else {
            eprintln!("Warning: GPU requested but no device provided. Using CPU mode.");
        }
    } else {
        println!("Particle system initialized in CPU mode (legacy)");
    }

    particle_system
}
