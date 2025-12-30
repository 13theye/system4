pub mod types;
pub use types::*;

use config::{Config, ConfigError, File};
use serde::Deserialize;
use std::env;

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub audience_window: AudienceWindowConfig,
    pub control_window: ControlWindowConfig,
    pub debug: DebugConfig,
    pub openai_service: OpenAIServiceConfig,
    pub osc_send: OscSendConfig,
    pub osc_loop: OscLoopConfig,
    pub osc_receive: OscReceiveConfig,
    pub particles: ParticleConfig,
    pub performer_window: PerformerWindowConfig,
    pub rendering: RenderConfig,
    pub speed: SpeedConfig,
}

impl Settings {
    /************************* Config file loading ********************/

    pub fn load() -> Result<Self, ConfigError> {
        // Get the executable's directory
        let exe_path = env::current_exe()
            .map_err(|e| ConfigError::Message(format!("Failed to get executable path: {}", e)))?;

        let exe_dir = exe_path.parent().ok_or_else(|| {
            ConfigError::Message("Failed to get executable directory".to_string())
        })?;

        // Build path to config file relative to executable
        let config_path = exe_dir.join("system4-support").join("config");

        let config_path_str = config_path
            .to_str()
            .ok_or_else(|| ConfigError::Message("Invalid config path".to_string()))?;

        let s = Config::builder()
            // Load configuration file from executable's directory
            .add_source(File::with_name(config_path_str).required(true))
            .build()?;

        // You can deserialize (and thus freeze) the entire configuration as
        s.try_deserialize()
    }
}
