// src/utils/id.rs
//
// Simple ID generator that just counts up from 0

pub struct IdGenerator {
    id_counter: usize,
}

impl Default for IdGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl IdGenerator {
    pub fn new() -> Self {
        Self { id_counter: 0 }
    }

    pub fn generate(&mut self) -> usize {
        self.id_counter += 1;
        self.id_counter
    }
}
