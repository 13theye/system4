// src/terminals/commands/drone.rs
//
// Re-exports of core drone types with app-specific extensions

pub use system4_core::commands::drone::{DroneBuilder, DroneConfig};

use crate::groups::VoiceId;
use crate::model::command_builder::DroneCommandBuilder;
use crate::model::controller::{Command, CommandInner, CommandSource, CompositeCommand};

/// Extension trait for DroneConfig providing app-specific functionality
pub trait DroneConfigExt {
    /// Get the voice ID as VoiceId enum
    fn get_voice_id(&self) -> VoiceId;

    /// Get default values for a specific voice
    fn get_defaults_for_voice(voice: VoiceId) -> DroneConfig;

    /// Merge this config with defaults, keeping specified values and using defaults for None values
    fn merge_with_defaults(self) -> DroneConfig;

    /// Generate parameter update commands from this config
    fn generate_parameter_commands(
        &self,
        voice_id: VoiceId,
        circle_id: usize,
        source: CommandSource,
    ) -> Vec<Command>;

    /// Generate only circle-specific parameter commands
    fn generate_circle_parameter_commands(
        &self,
        voice_id: VoiceId,
        circle_id: usize,
        source: CommandSource,
    ) -> Vec<Command>;

    /// Convert this DroneConfig to a CreateDrone Command
    fn to_create_command(&self, source: CommandSource) -> Command;

    /// Convert this DroneConfig to a ModifyDrone Command
    fn to_modify_command(&self, voice: VoiceId, source: CommandSource) -> Command;
}

impl DroneConfigExt for DroneConfig {
    fn get_voice_id(&self) -> VoiceId {
        VoiceId::from_i32(self.voice)
    }

    fn get_defaults_for_voice(voice: VoiceId) -> DroneConfig {
        // Override core defaults with app-specific defaults
        use system4_core::commands::drone::DroneConfig as CoreDroneConfig;

        let mut core_defaults = CoreDroneConfig::get_defaults_for_voice(voice.to_i32());

        // App-specific default overrides
        core_defaults.brightness = Some(1.0);
        core_defaults.volume = Some(1.0);
        core_defaults.feedback = Some(0.8);
        core_defaults.vibration = Some(0.2);
        core_defaults.outer_radius = Some(1600.0);
        core_defaults.inner_radius = Some(600.0);
        core_defaults.force = Some(4.0);
        core_defaults.noise = Some(0.1);

        core_defaults
    }

    fn merge_with_defaults(self) -> DroneConfig {
        let voice_id = self.get_voice_id();
        let defaults = <DroneConfig as DroneConfigExt>::get_defaults_for_voice(voice_id);
        // Use core's merge_with, passing the user config as the "other" to prefer user values
        defaults.merge_with(&self)
    }

    fn generate_parameter_commands(
        &self,
        voice_id: VoiceId,
        circle_id: usize,
        source: CommandSource,
    ) -> Vec<Command> {
        DroneCommandBuilder::generate_all_parameter_commands(self, voice_id, circle_id, source)
    }

    fn generate_circle_parameter_commands(
        &self,
        voice_id: VoiceId,
        circle_id: usize,
        source: CommandSource,
    ) -> Vec<Command> {
        DroneCommandBuilder::generate_circle_parameter_commands(self, voice_id, circle_id, source)
    }

    fn to_create_command(&self, source: CommandSource) -> Command {
        Command::new(
            CommandInner::Composite(CompositeCommand::CreateDrone {
                config: self.clone(),
            }),
            source,
        )
    }

    fn to_modify_command(&self, voice: VoiceId, source: CommandSource) -> Command {
        Command::new(
            CommandInner::Composite(CompositeCommand::ModifyDrone {
                voice_id: voice,
                config: self.clone(),
            }),
            source,
        )
    }
}
