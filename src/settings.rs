pub mod types;
pub use types::*;

use config::{Config, ConfigError, File};
use serde::Deserialize;
use std::env;

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub audience_window: AudienceWindowConfig,
    pub control_window: ControlWindowConfig,
    pub openai_service: OpenAIServiceConfig,
    pub osc_send: OscSendConfig,
    pub osc_loop: OscLoopConfig,
    pub osc_receive: OscReceiveConfig,
    pub particles: ParticleConfig,
    pub performer_window: PerformerWindowConfig,
    pub path: PathConfig,
    pub rendering: RenderConfig,
    pub tempo: TempoConfig,
    pub rhythm_vis: Option<RhythmVisConfig>,
}

impl Settings {
    /************************* Config file loading ********************/

    /// Returns the path to the config.toml file (used for saving).
    pub fn config_toml_path() -> Option<std::path::PathBuf> {
        let exe_path = env::current_exe().ok()?;
        let exe_dir = exe_path.parent()?;
        Some(exe_dir.join("system4-support").join("config.toml"))
    }

    /// Save rhythm vis colors to config.toml, replacing any existing [rhythm_vis.*] sections.
    pub fn save_rhythm_vis_colors(
        base: &RhythmColorConfig,
        grad1: &RhythmColorConfig,
        grad2: &RhythmColorConfig,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let path = Self::config_toml_path()
            .ok_or("Could not resolve config.toml path")?;
        let existing = std::fs::read_to_string(&path).unwrap_or_default();
        let stripped = Self::strip_rhythm_vis_section(&existing);
        let block = format!(
            "\n[rhythm_vis.base_color]\nr = {:.3}\ng = {:.3}\nb = {:.3}\na = {:.3}\n\n[rhythm_vis.gradient_1]\nr = {:.3}\ng = {:.3}\nb = {:.3}\na = {:.3}\n\n[rhythm_vis.gradient_2]\nr = {:.3}\ng = {:.3}\nb = {:.3}\na = {:.3}\n",
            base.r, base.g, base.b, base.a,
            grad1.r, grad1.g, grad1.b, grad1.a,
            grad2.r, grad2.g, grad2.b, grad2.a,
        );
        std::fs::write(&path, format!("{}{}", stripped.trim_end(), block))?;
        Ok(())
    }

    fn strip_rhythm_vis_section(content: &str) -> String {
        let mut result = String::new();
        let mut in_section = false;
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed == "[rhythm_vis]" || trimmed.starts_with("[rhythm_vis.") {
                in_section = true;
            } else if trimmed.starts_with('[') {
                in_section = false;
            }
            if !in_section {
                result.push_str(line);
                result.push('\n');
            }
        }
        result
    }

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
