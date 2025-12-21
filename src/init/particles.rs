use nannou::prelude::*;

use system4::particle::ParticleSystem;

const DEFAULT_PARTICLE_SIZE: f32 = 4.0;
const DEFAULT_PARTICLE_RGB: (f32, f32, f32) = (0.73, 0.73, 0.74);

pub fn init_particle_system(render_size: Vec2, particle_limit: u32) -> ParticleSystem {
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

    particle_system
}

/// Initialize GPU force field for particle system
///
/// Call this after the particle system is created and wgpu device is available.
/// This enables GPU-accelerated force field computation as an optional feature.
///
/// # Arguments
///
/// * `particle_system` - Mutable reference to the particle system
/// * `device` - WebGPU device for GPU resource creation
/// * `enable` - Whether to enable GPU computation immediately (default false for compatibility)
pub fn init_gpu_force_field(
    particle_system: &mut ParticleSystem,
    device: &wgpu::Device,
    enable: bool,
) {
    particle_system.init_gpu_force_field(device, enable);
}
