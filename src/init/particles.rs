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
