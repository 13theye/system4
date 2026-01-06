use crate::{
    command_engine::{commands::CommandSource, context::ExecutionContext, DroneCommandBuilder},
    forces::wind_circle::WindCircle,
    groups::VoiceId,
    terminals::commands::drone::DroneConfig,
};

#[derive(Debug, Default)]
pub struct CircleCommandHandler;

impl CircleCommandHandler {
    pub fn new() -> Self {
        Self
    }

    pub fn add_circle(
        &self,
        ctx: &mut dyn ExecutionContext,
        voice_id: VoiceId,
        circle_config: DroneConfig,
        source: CommandSource,
    ) {
        if !ctx.has_drone(voice_id) {
            println!("Error: Voice {:?} not found (NewCircle)", voice_id);
            return;
        }

        // Merge config with defaults
        let resolved_config = circle_config.merge_with_defaults();

        // Extract circle parameters (voice-level params are ignored for new circles)
        let gravity = resolved_config.gravity.unwrap();
        let force = resolved_config.force.unwrap();
        let outer_radius = resolved_config.outer_radius.unwrap();
        let inner_radius = resolved_config.inner_radius.unwrap();
        let center_x = resolved_config.center_x.unwrap();
        let center_y = resolved_config.center_y.unwrap();
        let noise = resolved_config.noise.unwrap();

        // Create the new WindCircle
        let voice = ctx.get_drone_mut(voice_id).unwrap();
        let circle_id = voice.issue_wind_circle_idx();

        let center = nannou::prelude::vec2(center_x, center_y);
        let circle = WindCircle::new(
            circle_id,
            voice_id,
            center,
            outer_radius,
            inner_radius,
            force,
            gravity,
            noise,
        );

        // Add the circle to the voice
        voice.add_wind_circle(circle.clone());

        // Queue parameter update commands for processing after this command completes
        let parameter_commands = DroneCommandBuilder::generate_circle_parameter_commands(
            &resolved_config,
            voice_id,
            circle_id,
            source,
        );

        for param_cmd in parameter_commands {
            ctx.queue_command(param_cmd);
        }
    }

    pub fn set_outer_radius(
        &self,
        ctx: &mut dyn ExecutionContext,
        voice_id: VoiceId,
        circle_id: usize,
        value: f32,
    ) {
        if !ctx.validate_circle_exists(voice_id, circle_id) {
            println!(
                "Error: Circle {}:{} not found",
                voice_id.to_i32(),
                circle_id
            );
            return;
        }

        if let Some(voice) = ctx.get_drone_mut(voice_id) {
            voice.set_circle_outer_radius(circle_id, value);
        }
    }

    pub fn set_inner_radius(
        &self,
        ctx: &mut dyn ExecutionContext,
        voice_id: VoiceId,
        circle_id: usize,
        value: f32,
    ) {
        if !ctx.validate_circle_exists(voice_id, circle_id) {
            println!(
                "Error: Circle {}:{} not found",
                voice_id.to_i32(),
                circle_id
            );
            return;
        }

        if let Some(voice) = ctx.get_drone_mut(voice_id) {
            voice.set_circle_inner_radius(circle_id, value);
        }
    }

    pub fn set_force(
        &self,
        ctx: &mut dyn ExecutionContext,
        voice_id: VoiceId,
        circle_id: usize,
        value: f32,
    ) {
        if !ctx.validate_circle_exists(voice_id, circle_id) {
            println!(
                "Error: Circle {}:{} not found",
                voice_id.to_i32(),
                circle_id
            );
            return;
        }

        let strength = value.min(30.0);
        if let Some(voice) = ctx.get_drone_mut(voice_id) {
            voice.set_circle_force(circle_id, strength);
        }
    }

    pub fn set_gravity(
        &self,
        ctx: &mut dyn ExecutionContext,
        voice_id: VoiceId,
        circle_id: usize,
        value: f32,
    ) {
        if !ctx.validate_circle_exists(voice_id, circle_id) {
            println!(
                "Error: Circle {}:{} not found",
                voice_id.to_i32(),
                circle_id
            );
            return;
        }

        if let Some(voice) = ctx.get_drone_mut(voice_id) {
            voice.set_circle_gravity(circle_id, value);
        }
    }

    pub fn set_noise(
        &self,
        ctx: &mut dyn ExecutionContext,
        voice_id: VoiceId,
        circle_id: usize,
        value: f32,
    ) {
        if !ctx.validate_circle_exists(voice_id, circle_id) {
            println!(
                "Error: Circle {}:{} not found",
                voice_id.to_i32(),
                circle_id
            );
            return;
        }

        if let Some(voice) = ctx.get_drone_mut(voice_id) {
            voice.set_circle_noise(circle_id, value);
        }
    }

    pub fn set_center_x(
        &self,
        ctx: &mut dyn ExecutionContext,
        voice_id: VoiceId,
        circle_id: usize,
        value: f32,
    ) {
        if !ctx.validate_circle_exists(voice_id, circle_id) {
            println!(
                "Error: Circle {}:{} not found",
                voice_id.to_i32(),
                circle_id
            );
            return;
        }

        if let Some(voice) = ctx.get_drone_mut(voice_id) {
            voice.set_circle_center_x(circle_id, value);
        }
    }

    pub fn set_center_y(
        &self,
        ctx: &mut dyn ExecutionContext,
        voice_id: VoiceId,
        circle_id: usize,
        value: f32,
    ) {
        if !ctx.validate_circle_exists(voice_id, circle_id) {
            println!(
                "Error: Circle {}:{} not found",
                voice_id.to_i32(),
                circle_id
            );
            return;
        }

        if let Some(voice) = ctx.get_drone_mut(voice_id) {
            voice.set_circle_center_y(circle_id, value);
        }
    }

    pub fn list_circles(&self, ctx: &dyn ExecutionContext, voice_id: VoiceId) {
        let circle_ids = self.get_wind_circle_ids(ctx, voice_id);
        let circles_str = if circle_ids.is_empty() {
            "No WindCircles found".to_string()
        } else {
            format!("WindCircle keys: {:?}", circle_ids)
        };

        let status_message = format!("Voice {} - {}", voice_id.to_i32(), circles_str);
        println!("{}", status_message);
    }

    pub fn remove_circle(&self, ctx: &mut dyn ExecutionContext, voice_id: VoiceId, circle_id: i32) {
        if !ctx.has_drone(voice_id) {
            println!("Error: Voice {:?} not found", voice_id);
            return;
        }

        // Use atomic operation that handles both wind field and voice removal
        if ctx.remove_circle_from_voice(voice_id, circle_id as usize) {
            let status_message = format!(
                "Voice {} - Removed WindCircle {}",
                voice_id.to_i32(),
                circle_id
            );
            println!("{}", status_message);
        }
    }

    fn get_wind_circle_ids(&self, ctx: &dyn ExecutionContext, voice: VoiceId) -> Vec<usize> {
        let Some(voice) = ctx.get_drone(voice) else {
            return Vec::new();
        };
        let mut ids: Vec<usize> = voice.wind_circles.keys().copied().collect();
        ids.sort();
        ids
    }
}
