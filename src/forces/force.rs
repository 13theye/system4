use crate::particle::Particle;

pub trait Force {
    fn apply(&self, particle: &mut Particle);
}
