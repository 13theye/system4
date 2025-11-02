# WARP.md

This file provides guidance to WARP (warp.dev) when working with code in this repository.

## Project Overview

System4 is a real-time audiovisual performance system written in Rust. It generates GPU-accelerated particle visualizations synchronized with audio using OSC (Open Sound Control) and Ableton Link clock synchronization. The system creates "voices" (audio-visual channels) with two main types:
- **Drones**: Continuous particle emissions controlled by wind force fields
- **Rhythms**: Rhythmic, sequenced visual patterns

The application runs multiple windows simultaneously: an audience window (high-res output), performer window (monitoring), and control window (parameter UI).

## Commands

### Build & Run
```bash
# Build the project
cargo build --release

# Run the application
cargo run --release

# Build without running
cargo build

# Check code without building
cargo check
```

### Development
```bash
# Run tests (if any exist)
cargo test

# Format code
cargo fmt

# Run linter
cargo clippy

# Clean build artifacts
cargo clean
```

### Configuration
- `config.toml` in project root configures OSC ports, window sizes, rendering parameters, BPM, and particle limits
- This file is copied to build output by `build.rs`
- Assets (fonts, etc.) are located in `assets/` directory

## Architecture

### Core Systems Overview

The application follows a **Model-View-Controller** pattern with specialized rendering:

1. **Model (`src/model/`)** - Central application state
   - Holds all voices, rhythms, particle system, clocks, sequencers, and OSC controllers
   - Contains unified command queue system with priority resolution (Terminal > OSC > UI)
   - Commands are processed through `controller.rs` which validates and executes them

2. **Particle System (`src/particle/`)** - Physics simulation
   - Manages particle lifecycle: emission, force application, movement, culling
   - Uses Structure-of-Arrays (SoA) pattern for GPU-friendly data layout
   - Parallel particle updates using Rayon for performance
   - Emitters generate particles at voice-specific positions
   - Forces (wind circles, gravity) affect particle trajectories

3. **Force Fields (`src/forces/`)** - Physics forces
   - **WindCircle**: Circular force field with inner/outer radius, configurable force strength
   - **WindField**: Spatial grid for efficient force lookups
   - Each voice can have multiple wind circles

4. **Groups (`src/groups/`)** - Voice abstraction
   - **Voice**: Represents a drone with emitters, wind circles, visual parameters (alpha, color, feedback)
   - **Rhythm**: Represents rhythmic sequencer with slots, wings, and subdivision parameters
   - **VoiceId**: Enum identifying voices (Voice0, Voice1, Voice2, Voice3)

5. **Rendering Pipeline (`nnpipe` library)** - GPU rendering
   - Multi-stage post-processing: particle rendering → heatmap → feedback effects → bloom → compositing
   - **ParticleRenderer**: Renders particles to texture
   - **HeatmapRenderer**: Creates heat map from particle positions
   - **SegmentRenderer**: Draws particle trails (feedback effect)
   - Multiple named textures in pipeline ("particles", "heatmap", "terminal", etc.)
   - TextureReshaper handles window-specific output

6. **Terminal System (`src/terminals/`)** - Command-based control
   - Text-based command parser with JavaScript-like syntax
   - Commands create/modify drones and rhythms: `makeDrone(1).brightness(0.5).begin();`
   - On-screen terminal view displays commands as they're typed
   - Drone parameter displays show current state for each voice

7. **Clock & Sequencing (`prat` library)** - Timing
   - ClockService provides Link-synchronized clock with configurable BPM, quantum, PPQN
   - SequencerService manages per-voice sequencers for rhythm playback
   - Sequencers send timing data via channels to rhythm instances

8. **OSC Communication (`src/osc/`)** - External control
   - Receives OSC messages on configured port for remote control
   - Sends OSC messages to audio engine (e.g., SuperCollider) for sound triggering
   - OSC loop sends messages back to self for testing

### Command System Architecture

**Priority-based command queueing** ensures consistent parameter updates:
- Commands from multiple sources (Terminal, OSC, UI sliders) are collected in a frame
- Before execution, conflicts are resolved: Terminal > OSC > UI (lower number = higher priority)
- Only the highest-priority command for each voice/parameter combination executes

**Command types:**
- **CompositeCommand**: Complex operations (CreateDrone, CreateRhythm, ModifyDrone, ModifyRhythm, NewCircle, Clear)
- **SimpleCommand**: Atomic parameter changes (Alpha, Volume, Feedback, OuterRadius, etc.)

**Command flow:**
1. Terminal/OSC/UI generates command
2. Command added to `model.command_queue`
3. `process_command_queue()` deduplicates by priority
4. `execute_command()` runs composite commands, which generate atomic parameter commands
5. Atomic commands are queued again and executed in next cycle
6. Terminal display updated via `TerminalViewManager`

### Voice Structure

Each **Voice** contains:
- **Emitters** (LinearEmitter): Spawn particles along left/right edges of force field bounds
- **WindCircles**: Force fields that push/pull particles
- **Parameters**: alpha_limit (brightness), volume, feedback (trail length), vibration (position jitter), segment_length
- Voice bounds calculated from all wind circle extents

**Voice initialization** (two-phase):
1. Structure creation: WindCircle + emitters established
2. Parameter application: Commands set brightness, volume, force, gravity, noise, etc.

### Rhythm System

**Rhythms** organize sequenced events:
- **Slots**: Individual events with length, velocity, cutoff parameters
- **Wings**: Groupings of slots (e.g., wing 0 contains slots [0, 4, 8], wing 1 contains slots [1, 5, 9])
- **Subdivision**: Timing grid (Sixteenth, Eighth, Quarter, etc.)
- **Capacity**: Total number of slots

**RhythmView** renders rhythm as visual formations:
- **RhythmCircleFormation**: Circular arrangement
- **RhythmLinesFormation**: Linear arrangement
- Active slots highlighted during playback

### Data Flow

```
User Input (Terminal/OSC/UI)
    ↓
Command Generation
    ↓
Command Queue (priority resolution)
    ↓
Model Update (voices/rhythms/particles)
    ↓
Particle System Update (emit/force/move)
    ↓
GPU Buffer Preparation (Vec<ParticleGpu>, Vec<SegmentGpu>)
    ↓
Rendering Pipeline (textures → effects → composite)
    ↓
Window Output (audience/performer/control)
```

## Key Patterns & Conventions

### Module Organization
- Each subsystem has `mod.rs` exporting public API
- Related functionality grouped in submodules (e.g., `terminals/commands/`, `terminals/parsing/`)
- View layer separated from model logic

### Borrow Checker Management
- `RefCell<T>` used for interior mutability: `rendering: RefCell<Nnpipe>`, `terminal_manager: RefCell<TerminalViewManager>`
- Pre-compute values before entering loops that need mutable borrows
- HashMap-based storage for voices/rhythms enables independent access

### Configuration Pattern
All subsystems follow merge-with-defaults pattern:
- User provides partial configuration (e.g., `DroneConfig` with some `None` fields)
- `merge_with_defaults()` fills missing values with voice-specific or global defaults
- Enables terse commands while maintaining deterministic behavior

### GPU Data Layout
Particles use SoA (Structure of Arrays) for cache-friendly GPU uploads:
- Separate arrays for positions, colors, sizes rather than array of structs
- `ParticleGpu` struct packs data for shader consumption
- Segment trails similarly structured as `SegmentGpu`

### Parallel Processing
- Particle updates use Rayon's `par_iter_mut()` for parallel physics computation
- Force field lookups optimized with spatial grid
- GPU rendering fully parallelized by hardware

## Common Development Patterns

### Adding a New Voice Parameter

1. Add field to `VoiceParams` struct in `src/groups/voice.rs`
2. Add setter method to `Voice` impl
3. Create `SimpleCommand` variant in `src/model/controller.rs`
4. Add command execution case in `execute_simple_command()`
5. Add parsing support in `src/terminals/parsing/drone_parser.rs`
6. Add UI slider in `update_control_ui()` in `src/main.rs`
7. Add OSC message handler in `src/osc/osc_control.rs`
8. Add to `DroneCommandBuilder` in `src/model/command_builder.rs`

### Adding a New Force Type

1. Create force struct in `src/forces/` implementing force calculation
2. Add to `ForceFields` struct
3. Register with wind field spatial grid
4. Add creation command and parameters
5. Wire up to terminal parser and UI

### Debugging Rendering Issues

- Enable debug views: Press `P` for bounds overlay
- Check texture names match in pipeline: "particles", "heatmap", "terminal", etc.
- Verify texture formats (Rgba8UnormSrgb vs Rgba16Float)
- Ensure `encode_clear_all_textures()` called before drawing
- Check reshaper matches window in view functions

### Performance Optimization

- Particle limit configurable in `config.toml`
- Spatial grid resolution affects force lookup speed (set in `ParticleSystem::new()`)
- Parallel updates already enabled via Rayon
- GPU buffer reuse avoids allocations (see `gpu_particle_buffer.clear()`)

## Dependencies

- **nannou**: Creative coding framework, windowing, drawing
- **nnpipe**: Custom rendering pipeline library (workspace dependency)
- **prat**: Clock and sequencing library (workspace dependency)
- **nannou_osc**: OSC protocol support
- **egui/nannou_egui**: Immediate mode GUI for control window
- **rayon**: Parallel iterators
- **rusty_link**: Ableton Link clock sync
- **rand/rand_distr**: Random number generation for particle variation

## Testing & Workflow Notes

- No automated tests currently present in codebase
- Testing done through live performance use
- OSC loopback feature (`osc_loop` in config) enables self-testing
- Terminal examples accessible via Examples panel in control window
- Use `cargo check` for fast compile-time validation during development
