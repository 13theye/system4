use crate::{
    command_engine::{commands::CommandSource, context::ExecutionContext, RhythmCommandBuilder},
    groups::{Rhythm, Voice, VoiceId},
    terminals::commands::rhythm::{RangeSize, RhythmConfig, RhythmParamModification},
    view::rhythm::RhythmFormationType,
};
use std::time::Instant;

#[derive(Default)]
pub struct RhythmCommandHandler;

impl RhythmCommandHandler {
    pub fn new() -> Self {
        Self
    }

    pub fn create_rhythm(
        &self,
        ctx: &mut dyn ExecutionContext,
        config: RhythmConfig,
        source: CommandSource,
        now: Instant,
    ) {
        let voice_id = config.voice;

        if ctx.has_rhythm(voice_id) || ctx.has_drone(voice_id) {
            println!("Controller: Voice {} already exists", voice_id);
            return;
        }

        // Merge config with defaults
        let resolved_config = config.merge_with_defaults();

        // Phase 1: Create basic rhythm structure
        let params = resolved_config.to_rhythm_params();
        let mut rhythm = Rhythm::new_with_params(voice_id, params);

        // Initialize slots and wings
        rhythm.initialize_slots(ctx.rng());
        rhythm.randomize_wings(ctx.rng());

        // Start sequencer
        rhythm.add_sequencer(ctx.sequencer_service());

        // Subscribe to sequencer callbacks
        if let Some(data_rx) = ctx.sequencer_service().get_data_rx(voice_id) {
            rhythm.set_sequencer_data_rx(data_rx);
        }

        let radius = if voice_id == VoiceId::Voice1 {
            800.0
        } else {
            450.0
        };

        // Create the RhythmFormation
        ctx.rhythm_view_mut().add_formation(
            voice_id,
            RhythmFormationType::Circle { radius },
            rhythm.get_params(),
            now,
        );

        // Insert rhythm before applying parameters so validation can find it
        let voice = Voice::new_from_rhythm(rhythm);
        ctx.insert_voice(voice_id, voice);

        // Phase 2: Apply parameters through the command pipeline
        let parameter_commands = RhythmCommandBuilder::generate_all_parameter_commands(
            &resolved_config,
            voice_id,
            source,
        );

        // Queue parameter commands to avoid recursive execution
        for param_cmd in parameter_commands {
            ctx.queue_command(param_cmd);
        }

        // Phase 3: Start all sequencers to sync on the next beat
        ctx.sequencer_service().start_all();

        let status_message = format!("Voice {} - Created rhythm sequencer", voice_id.to_i32());
        println!("{}", status_message);
    }

    pub fn modify_rhythm(
        &self,
        ctx: &mut dyn ExecutionContext,
        voice_id: VoiceId,
        config: RhythmConfig,
        source: CommandSource,
        _now: Instant,
    ) {
        if !ctx.has_rhythm(voice_id) {
            println!(
                "Error: Rhythm not found for Voice {:?} (ModifyRhythm)",
                voice_id
            );
            return;
        }

        // Generate and queue parameter commands
        let parameter_commands =
            RhythmCommandBuilder::generate_all_parameter_commands(&config, voice_id, source);

        for cmd in parameter_commands {
            ctx.queue_command(cmd);
        }
    }

    pub fn set_capacity(
        &self,
        ctx: &mut dyn ExecutionContext,
        voice_id: VoiceId,
        value: usize,
        now: Instant,
    ) {
        if value < 1 {
            return;
        }

        // First update rhythm capacity
        {
            if let Some(rhythm) = ctx.get_rhythm_mut(voice_id) {
                rhythm.set_capacity(value);
            } else {
                return;
            }
        }

        // Update sequencer
        ctx.update_rhythm_sequencer(voice_id);

        // Then reinitialize formation
        let params = ctx.get_rhythm(voice_id).map(|r| r.get_params().clone());
        if let Some(params) = params {
            ctx.rhythm_view_mut()
                .reinitialize_formation(voice_id, &params, now);
        }
    }

    pub fn set_num_wings(
        &self,
        ctx: &mut dyn ExecutionContext,
        voice_id: VoiceId,
        value: usize,
        now: Instant,
    ) {
        // First update rhythm
        {
            if let Some(rhythm) = ctx.get_rhythm_mut(voice_id) {
                rhythm.set_num_wings(value);
            } else {
                return;
            }
        }

        // Reroll wings
        ctx.rhythm_reroll_wings(voice_id);

        // Then reinitialize formation
        let params = ctx.get_rhythm(voice_id).map(|r| r.get_params().clone());
        if let Some(params) = params {
            ctx.rhythm_view_mut()
                .reinitialize_formation(voice_id, &params, now);
        }
    }

    pub fn set_subdivision(
        &self,
        ctx: &mut dyn ExecutionContext,
        voice_id: VoiceId,
        value: prat::BeatSubdivision,
    ) {
        {
            if let Some(rhythm) = ctx.get_rhythm_mut(voice_id) {
                rhythm.set_subdivision(value);
            }
        }
        ctx.update_rhythm_sequencer(voice_id);
    }

    pub fn add_wings(
        &self,
        ctx: &mut dyn ExecutionContext,
        voice_id: VoiceId,
        count: usize,
        now: Instant,
    ) {
        // Check if rhythm exists
        if !ctx.has_rhythm(voice_id) {
            let error_message =
                format!("Voice {} has no rhythm to add wings to", voice_id.to_i32());
            println!("Error: {}", error_message);
            return;
        }

        // Add wings and update sequencer
        ctx.rhythm_add_wings(voice_id, count);
        ctx.update_rhythm_sequencer(voice_id);

        // Get wing count
        let wing_count = {
            if let Some(rhythm) = ctx.get_rhythm(voice_id) {
                rhythm.get_params().wings.len()
            } else {
                return;
            }
        };

        // Then reinitialize formation
        let params = ctx.get_rhythm(voice_id).map(|r| r.get_params().clone());
        if let Some(params) = params {
            ctx.rhythm_view_mut()
                .reinitialize_formation(voice_id, &params, now);

            let status_message = format!(
                "Voice {} - Added {} wings (total: {})",
                voice_id.to_i32(),
                count,
                wing_count
            );
            println!("{}", status_message);
        }
    }

    pub fn remove_wings(
        &self,
        ctx: &mut dyn ExecutionContext,
        voice_id: VoiceId,
        count: usize,
        now: Instant,
    ) {
        // First remove wings
        {
            if let Some(rhythm) = ctx.get_rhythm_mut(voice_id) {
                rhythm.remove_wings(count);
            } else {
                let error_message = format!(
                    "Voice {} has no rhythm to remove wings from",
                    voice_id.to_i32()
                );
                println!("Error: {}", error_message);
                return;
            }
        }

        // Update sequencer
        ctx.update_rhythm_sequencer(voice_id);

        // Get wing count
        let wing_count = {
            if let Some(rhythm) = ctx.get_rhythm(voice_id) {
                rhythm.get_params().wings.len()
            } else {
                return;
            }
        };

        // Then reinitialize formation
        let params = ctx.get_rhythm(voice_id).map(|r| r.get_params().clone());
        if let Some(params) = params {
            ctx.rhythm_view_mut()
                .reinitialize_formation(voice_id, &params, now);

            let status_message = format!(
                "Voice {} - Removed {} wings (total: {})",
                voice_id.to_i32(),
                count,
                wing_count
            );
            println!("{}", status_message);
        }
    }

    pub fn clear_rhythm(&self, ctx: &mut dyn ExecutionContext, voice_id: VoiceId, now: Instant) {
        // First check if rhythm exists, then stop the sequencer
        {
            if !ctx.has_rhythm(voice_id) {
                let error_message = format!("Voice {} has no rhythm to clear", voice_id.to_i32());
                println!("Error: {}", error_message);
                return;
            }
        }

        // Stop the sequencer
        ctx.rhythm_stop_sequencer(voice_id);

        // Trigger clearing animation in view
        ctx.rhythm_view_mut().clear_formation(voice_id, now);

        // Remove the rhythm from the model (view continues animating)
        ctx.remove_voice(voice_id);

        let status_message = format!(
            "Voice {} - Cleared rhythm and stopped sequencer",
            voice_id.to_i32()
        );
        println!("{}", status_message);
    }

    pub fn set_length_range(
        &self,
        ctx: &mut dyn ExecutionContext,
        voice_id: VoiceId,
        range: RangeSize,
    ) {
        if let Some(rhythm) = ctx.get_rhythm_mut(voice_id) {
            rhythm.set_length_range(range);
        }
    }

    pub fn set_velocity_range(
        &self,
        ctx: &mut dyn ExecutionContext,
        voice_id: VoiceId,
        range: RangeSize,
    ) {
        if let Some(rhythm) = ctx.get_rhythm_mut(voice_id) {
            rhythm.set_velocity_range(range);
        }
    }

    pub fn set_cutoff_range(
        &self,
        ctx: &mut dyn ExecutionContext,
        voice_id: VoiceId,
        range: RangeSize,
    ) {
        if let Some(rhythm) = ctx.get_rhythm_mut(voice_id) {
            rhythm.set_cutoff_range(range);
        }
    }

    pub fn modify_length(
        &self,
        ctx: &mut dyn ExecutionContext,
        voice_id: VoiceId,
        modification: RhythmParamModification,
    ) {
        ctx.rhythm_modify_all_slots_length(voice_id, modification);
        ctx.update_rhythm_sequencer(voice_id);
    }

    pub fn modify_velocity(
        &self,
        ctx: &mut dyn ExecutionContext,
        voice_id: VoiceId,
        modification: RhythmParamModification,
    ) {
        ctx.rhythm_modify_all_slots_velocity(voice_id, modification);
        ctx.update_rhythm_sequencer(voice_id);
    }

    pub fn modify_cutoff(
        &self,
        ctx: &mut dyn ExecutionContext,
        voice_id: VoiceId,
        modification: RhythmParamModification,
    ) {
        ctx.rhythm_modify_all_slots_cutoff(voice_id, modification);
        ctx.update_rhythm_sequencer(voice_id);
    }
}
