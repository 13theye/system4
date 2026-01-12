use crate::{
    command_engine::{commands::CommandSource, context::ExecutionContext, DroneCommandBuilder},
    groups::{Drone, Voice, VoiceId},
    terminals::commands::drone::DroneConfig,
};
use std::time::Instant;

#[derive(Default)]
pub struct DroneCommandHandler;

impl DroneCommandHandler {
    pub fn new() -> Self {
        Self
    }

    pub fn create_drone(
        &self,
        ctx: &mut dyn ExecutionContext,
        config: DroneConfig,
        source: CommandSource,
        _now: Instant,
    ) {
        let voice_id = config.voice;

        if ctx.has_rhythm(voice_id) || ctx.has_drone(voice_id) {
            println!("Controller: Voice {} already exists", voice_id);
            return;
        }

        // Merge config with defaults
        let resolved_config = config.merge_with_defaults();

        // Phase 1: Initialize drone structure (WindCircle and emitters)
        let mut drone = Drone::new_with_id(voice_id);

        // Get particle system defaults
        let default_color = ctx.default_particle_color();
        let global_max_spawn_rate = ctx.global_max_spawn_rate();

        let circle_id =
            drone.initialize_drone(&resolved_config, default_color, global_max_spawn_rate);

        ctx.osc_send()
            .send_drone_on_off(resolved_config.voice.to_i32(), 1);
        drone.set_is_spawning(true);

        // Insert voice before applying parameters so validation can find it
        let voice = Voice::new_from_drone(drone);
        ctx.insert_voice(voice_id, voice);

        // Phase 2: Apply parameters through the command pipeline
        let parameter_commands = DroneCommandBuilder::generate_all_parameter_commands(
            &resolved_config,
            voice_id,
            circle_id,
            source,
        );

        // Queue parameter commands to avoid recursive execution
        for param_cmd in parameter_commands {
            ctx.queue_command(param_cmd);
        }
    }

    pub fn modify_drone(
        &self,
        ctx: &mut dyn ExecutionContext,
        voice_id: VoiceId,
        config: DroneConfig,
        source: CommandSource,
        _now: Instant,
    ) {
        if !ctx.has_drone(voice_id) {
            println!("Error: Voice {:?} not found (ModifyDrone)", voice_id);
            return;
        }

        // Generate atomic commands for voice-level parameters
        let parameter_commands =
            DroneCommandBuilder::generate_voice_parameter_commands(&config, voice_id, source);

        // Queue parameter commands
        for cmd in parameter_commands {
            ctx.queue_command(cmd);
        }

        // Circle-level parameters require circle_id, which ModifyDrone doesn't specify
        // These should be handled by explicit circle commands instead
    }

    pub fn set_alpha(&self, ctx: &mut dyn ExecutionContext, voice_id: VoiceId, value: f32) {
        if !ctx.has_drone(voice_id) {
            println!("Error: Voice {:?} not found (Alpha)", voice_id);
            return;
        }

        if let Some(voice) = ctx.get_drone_mut(voice_id) {
            voice.set_alpha_limit(value);
        }
    }

    pub fn set_volume(&self, ctx: &mut dyn ExecutionContext, voice_id: VoiceId, value: f32) {
        if !ctx.has_drone(voice_id) {
            println!("Error: Voice {:?} not found (Volume)", voice_id);
            return;
        }

        if let Some(voice) = ctx.get_drone_mut(voice_id) {
            voice.set_volume(value);
        }
    }

    pub fn set_feedback(&self, ctx: &mut dyn ExecutionContext, voice_id: VoiceId, value: f32) {
        if !ctx.has_drone(voice_id) {
            println!("Error: Voice {:?} not found (Feedback)", voice_id);
            return;
        }

        if let Some(voice) = ctx.get_drone_mut(voice_id) {
            voice.set_feedback(value);
        }
    }

    pub fn set_vibration(&self, ctx: &mut dyn ExecutionContext, voice_id: VoiceId, value: f32) {
        if !ctx.has_drone(voice_id) {
            println!("Error: Voice {:?} not found (Vibration)", voice_id);
            return;
        }

        if let Some(voice) = ctx.get_drone_mut(voice_id) {
            voice.set_vibration(value);
        }
    }

    pub fn move_emitters(&self, ctx: &mut dyn ExecutionContext, voice_id: VoiceId, value: f32) {
        if !ctx.has_drone(voice_id) {
            println!("Error: Voice {:?} not found (MoveEmitters)", voice_id);
            return;
        }

        if let Some(voice) = ctx.get_drone_mut(voice_id) {
            voice.set_emitter_position(value);
        }
    }

    pub fn clear_drone(&self, ctx: &mut dyn ExecutionContext, voice_id: VoiceId) {
        if let Some(drone) = ctx.get_drone_mut(voice_id) {
            // Mark voice as clearing (will be removed when all particles are dead)
            drone.state = crate::groups::DroneState::Clearing;
            // Stop spawning new particles
            drone.set_is_spawning(false);
        }

        // Mark all particles to fade out
        ctx.fade_out_all_particles(voice_id);

        // Remove circles from wind field
        ctx.remove_all_circles_from_voice(voice_id);

        // OSC notification
        ctx.osc_send().send_drone_on_off(voice_id.to_i32(), 0);
    }
}
