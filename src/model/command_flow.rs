//! src/model/command_flow.rs
//!
//! Command-flow helpers: enqueue commands and process one "tick" of execution.

use crate::{
    command_engine::{Command, CommandEngine},
    groups::VoiceId,
    model::Model,
};

use std::time::Instant;

impl Model {
    /// Queue a command for later processing.
    pub fn queue_command(&mut self, command: Command) {
        self.command_queue.push(command);
    }

    /// Hard-remove a voice immediately (used by shutdown paths).
    pub fn kill_voice(&mut self, voice_id: VoiceId) {
        if self.voice_manager.get_voice_mut(voice_id).is_none() {
            return;
        };

        // Remove the voice from the hashmap
        self.voice_manager.remove_voice(voice_id);
    }

    /// Process all queued commands with priority resolution (Terminal > OSC > UI)
    /// using `CommandEngine`.
    pub fn process_command_queue(&mut self, now: Instant) {
        // Reset per-frame auto-AI flag; it will be set by `ExecutionContext::log_command`.
        self.auto_ai_pending_for_voice1 = false;

        // Drain the current queue snapshot (any commands queued during execution
        // will run next tick).
        let commands = std::mem::take(&mut self.command_queue);
        CommandEngine::new().process_commands(self, commands, now);

        // After all commands have been applied, trigger at most one AI rhythm
        // request using the final Voice1 rhythm state, if requested.
        if self.auto_ai_pending_for_voice1
            && self.ui_state.auto_ai_from_voice1
            && self.rhythm_manager.has_rhythm(VoiceId::Voice1)
            && !self.rhythm_manager.is_ai_request_pending()
        {
            self.rhythm_manager.request_ai_rhythm();
        }
    }
}
