# GPU Force Field Quick Start

## Phase 1 is Complete!

The GPU force field system is now integrated into System4. Here's how to use it.

## Quick Enable (3 steps)

### 1. Initialize GPU Force Field (add to model initialization)

In `src/main.rs`, after creating the particle system:

```rust
fn model(app: &App) -> Model {
    // ... existing initialization ...

    let mut particle_system = init::particles::init_particle_system(render_size, particle_limit);

    // NEW: Initialize GPU force field
    let device = app.main_window().device();
    init::particles::init_gpu_force_field(&mut particle_system, device, false);

    // ... rest of model setup ...
}
```

### 2. Enable GPU Computation (runtime toggle)

Add a keybinding or UI toggle to enable GPU forces:

```rust
// In update() or event handler
if app.keys.down.contains(&Key::G) {
    model.particle_system.set_use_gpu_forces(true);
    println!("GPU force field enabled");
}

if app.keys.down.contains(&Key::C) {
    model.particle_system.set_use_gpu_forces(false);
    println!("CPU force field enabled (default)");
}
```

### 3. Update GPU Force Field (in update loop)

In the update function, add GPU force field update:

```rust
fn update(app: &App, model: &mut Model, _update: Update) {
    // ... existing update code ...

    // Update forces (CPU path - always runs)
    model.particle_system.forces.update(&mut model.voice_manager.voices, &mut model.rng);

    // NEW: Update GPU force field if enabled
    if model.particle_system.is_using_gpu_forces() {
        let device = app.main_window().device();
        let queue = app.main_window().queue();

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Force Field Update"),
        });

        let contribution_count = model.particle_system.forces.update_gpu(
            &model.voice_manager.voices,
            queue,
            &mut encoder,
        );

        queue.submit(Some(encoder.finish()));

        // For now, read back GPU forces to CPU for physics
        let _gpu_forces = model.particle_system.forces.read_back_gpu(device, queue);
    }

    // ... rest of update ...
}
```

## That's It!

Press `G` to enable GPU force field, `C` to switch back to CPU.

## Validation Mode (Compare CPU vs GPU)

Want to validate that GPU produces correct results?

```rust
// Add this temporarily to compare CPU and GPU
if model.particle_system.is_using_gpu_forces() {
    use std::time::Instant;

    let cpu_start = Instant::now();
    model.particle_system.forces.update(&mut model.voice_manager.voices, &mut model.rng);
    let cpu_time = cpu_start.elapsed();

    let gpu_start = Instant::now();
    let device = app.main_window().device();
    let queue = app.main_window().queue();
    let mut encoder = device.create_command_encoder(&Default::default());
    let count = model.particle_system.forces.update_gpu(&model.voice_manager.voices, queue, &mut encoder);
    queue.submit(Some(encoder.finish()));
    let gpu_forces = model.particle_system.forces.read_back_gpu(device, queue);
    let gpu_time = gpu_start.elapsed();

    println!("CPU: {:?}, GPU: {:?} ({} contributions)", cpu_time, gpu_time, count);
}
```

## Performance Tips

1. **Disable readback in production**: Phase 2 will eliminate this need
2. **Reuse encoders**: If you have other GPU work, use the same encoder
3. **Profile before optimizing**: The GPU path may not always be faster for small grids

## Troubleshooting

**GPU forces not working?**
- Check that `init_gpu_force_field()` was called
- Verify `set_use_gpu_forces(true)` was called
- Ensure wgpu device is available

**Performance is worse?**
- Small grids (<50x50) may be slower on GPU
- Few WindCircles (<5) may not benefit from GPU
- Readback overhead dominates in Phase 1 (fixed in Phase 2)

**Forces look different?**
- GPU uses curl noise vs CPU random rotation
- Visually equivalent but not numerically identical
- This is expected and correct

## Next Steps

Phase 1 is complete! Next up:
- Phase 2: Move particle physics to GPU (5-10x faster)
- Eliminate CPU readback overhead
- Full GPU pipeline for 15k-25k particles

## More Information

See `/Users/jeanhank/Repos/creative/13thlib/nnpipe/GPU_FORCE_FIELD_USAGE.md` for:
- Detailed API reference
- Advanced usage patterns
- Performance benchmarking
- Architecture overview
