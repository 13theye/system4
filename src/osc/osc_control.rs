// src/osc/osc_control.rs
//
// OSC commands, sender, and receiver for System3

use nannou_osc as osc;
use std::error::Error;

use crate::config::OscSendConfig;

#[derive(Debug)]
pub enum OscCommand {
    ParticlesAlpha {
        id: i32,
        val: f32,
    },
    ParticlesNumParticles {
        id: i32,
        val: f32,
    },
    ParticlesForce {
        id: i32,
        val: f32,
    },
    ParticlesInnerRadius {
        id: i32,
        val: f32,
    },
    ParticlesOuterRadius {
        id: i32,
        val: f32,
    },
    ParticlesGravity {
        id: i32,
        val: f32,
    },
    ParticlesTrail {
        id: i32,
        val: f32,
    },
    
    // GPU-specific advanced parameters
    ParticlesColorTint {
        id: i32,
        r: f32,
        g: f32,
        b: f32,
    },
    ParticlesSizeMultiplier {
        id: i32,
        val: f32,
    },
    ParticlesPhysicsScale {
        id: i32,
        val: f32,
    },
    ParticlesGroupVisible {
        id: i32,
        visible: bool,
    },
    ParticlesSpawnRateFactor {
        id: i32,
        val: f32,
    },
    
    // Batch parameter updates for efficiency
    ParticlesBatchUpdate {
        id: i32,
        alpha: Option<f32>,
        size_multiplier: Option<f32>,
        trail: Option<f32>,
        physics_scale: Option<f32>,
        spawn_rate_factor: Option<f32>,
        visible: Option<bool>,
    },
    
    // Debug visualization commands
    ParticlesDebugEnable {
        enabled: bool,
    },
    ParticlesDebugToggle {
        feature: String,
        enabled: bool,
    },

    EraseDrone {
        id: i32,
    },
    MakeDrone {
        id: i32,
        alpha: i32,
        num_particles: i32,
        force: i32,
        gravity: i32,
        trail: i32,
    },

    TermBrightness {
        id: i32,
        val: f32,
    },
    TermVolume {
        id: i32,
        val: f32,
    },
    TermForce {
        id: i32,
        val: f32,
    },
    TermShake {
        id: i32,
        val: f32,
    },
    TermFeedback {
        id: i32,
        val: f32,
    },
    TermClear {
        id: i32,
    },

    TermDroneOnOff {
        id: i32,
        val: i32,
    },
}

pub struct OscSender {
    sender: osc::Sender,
    target_addr: String,
    target_port: u16,
}

impl OscSender {
    pub fn new(config: &OscSendConfig) -> Result<Self, Box<dyn Error>> {
        let target_addr = config.target_addr.to_owned();
        let target_port = config.target_port;
        let sender = osc::sender()?;
        println!("OSC Sender sending to {}:{}", target_addr, target_port);

        Ok(Self {
            sender,
            target_addr,
            target_port,
        })
    }

    // Callback message when drone initialization is done
    pub fn send_drone_on_off(&self, player_id: i32, val: i32) {
        let addr = "/sys2/droneOnOff".to_string();
        let args = vec![osc::Type::Int(player_id), osc::Type::Int(val)];
        self.sender
            .send((addr, args), (self.target_addr.as_str(), self.target_port))
            .ok();
    }

    #[allow(clippy::too_many_arguments)]
    pub fn send_make_drone(
        &self,
        player_id: i32,
        alpha: i32,
        num_particles: i32,
        force: i32,
        gravity: i32,
        trail: i32,
    ) {
        println!(
            "Sending makeDrone to {}:{}",
            self.target_addr, self.target_port
        );
        let addr = "/sys2/makeDrone".to_string();
        let args = vec![
            osc::Type::Int(player_id),
            osc::Type::Int(alpha),
            osc::Type::Int(num_particles),
            osc::Type::Int(force),
            osc::Type::Int(gravity),
            osc::Type::Int(trail),
        ];
        self.sender
            .send((addr, args), (self.target_addr.as_str(), self.target_port))
            .ok();
    }

    pub fn send_erase_drone(&self, player_id: i32) {
        let addr = "/sys2/eraseDrone".to_string();
        let args = vec![osc::Type::Int(player_id)];
        self.sender
            .send((addr, args), (self.target_addr.as_str(), self.target_port))
            .ok();
    }

    pub fn send_inner_radius(&self, player_id: i32, val: f32) {
        let addr = "/sys2/particles/innerRadius".to_string();
        let args = vec![osc::Type::Int(player_id), osc::Type::Float(val)];
        self.sender
            .send((addr, args), (self.target_addr.as_str(), self.target_port))
            .ok();
    }

    pub fn send_outer_radius(&self, player_id: i32, val: f32) {
        let addr = "/sys2/particles/outerRadius".to_string();
        let args = vec![osc::Type::Int(player_id), osc::Type::Float(val)];
        self.sender
            .send((addr, args), (self.target_addr.as_str(), self.target_port))
            .ok();
    }
}

pub struct OscController {
    command_queue: Vec<OscCommand>,
    receiver: osc::Receiver,
}

impl OscController {
    pub fn new(port: u16) -> Result<Self, Box<dyn Error>> {
        let receiver = osc::receiver(port)?;

        Ok(Self {
            command_queue: Vec::new(),
            receiver,
        })
    }

    pub fn process_messages(&mut self) {
        for (packet, _addr) in self.receiver.try_iter() {
            for message in packet.into_msgs() {
                match message.addr.as_str() {
                    /********************* Particle Commands *************************** */
                    "/sys2/particles/alpha" => {
                        if let [osc::Type::Int(id), osc::Type::Float(val)] = &message.args[..] {
                            self.command_queue
                                .push(OscCommand::ParticlesAlpha { id: *id, val: *val });
                        }
                    }
                    "/sys2/particles/numParticles" => {
                        if let [osc::Type::Int(id), osc::Type::Float(val)] = &message.args[..] {
                            self.command_queue
                                .push(OscCommand::ParticlesNumParticles { id: *id, val: *val });
                        }
                    }
                    "/sys2/particles/force" => {
                        if let [osc::Type::Int(id), osc::Type::Float(val)] = &message.args[..] {
                            self.command_queue
                                .push(OscCommand::ParticlesForce { id: *id, val: *val });
                        }
                    }
                    "/sys2/particles/innerRadius" => {
                        if let [osc::Type::Int(id), osc::Type::Float(val)] = &message.args[..] {
                            self.command_queue
                                .push(OscCommand::ParticlesInnerRadius { id: *id, val: *val });
                        }
                    }
                    "/sys2/particles/outerRadius" => {
                        if let [osc::Type::Int(id), osc::Type::Float(val)] = &message.args[..] {
                            self.command_queue
                                .push(OscCommand::ParticlesOuterRadius { id: *id, val: *val });
                        }
                    }
                    "/sys2/particles/gravity" => {
                        if let [osc::Type::Int(id), osc::Type::Float(val)] = &message.args[..] {
                            self.command_queue
                                .push(OscCommand::ParticlesGravity { id: *id, val: *val });
                        }
                    }
                    "/sys2/particles/trail" => {
                        if let [osc::Type::Int(id), osc::Type::Float(val)] = &message.args[..] {
                            self.command_queue
                                .push(OscCommand::ParticlesTrail { id: *id, val: *val });
                        }
                    }
                    "/sys2/particles/colorTint" => {
                        if let [osc::Type::Int(id), osc::Type::Float(r), osc::Type::Float(g), osc::Type::Float(b)] = &message.args[..] {
                            self.command_queue
                                .push(OscCommand::ParticlesColorTint { id: *id, r: *r, g: *g, b: *b });
                        }
                    }
                    "/sys2/particles/sizeMultiplier" => {
                        if let [osc::Type::Int(id), osc::Type::Float(val)] = &message.args[..] {
                            self.command_queue
                                .push(OscCommand::ParticlesSizeMultiplier { id: *id, val: *val });
                        }
                    }
                    "/sys2/particles/physicsScale" => {
                        if let [osc::Type::Int(id), osc::Type::Float(val)] = &message.args[..] {
                            self.command_queue
                                .push(OscCommand::ParticlesPhysicsScale { id: *id, val: *val });
                        }
                    }
                    "/sys2/particles/visible" => {
                        if let [osc::Type::Int(id), osc::Type::Int(visible)] = &message.args[..] {
                            self.command_queue
                                .push(OscCommand::ParticlesGroupVisible { id: *id, visible: *visible != 0 });
                        }
                    }
                    "/sys2/particles/spawnRateFactor" => {
                        if let [osc::Type::Int(id), osc::Type::Float(val)] = &message.args[..] {
                            self.command_queue
                                .push(OscCommand::ParticlesSpawnRateFactor { id: *id, val: *val });
                        }
                    }
                    "/sys2/particles/batchUpdate" => {
                        if let [osc::Type::Int(id), args @ ..] = &message.args[..] {
                            // Parse optional parameters for batch update
                            let mut alpha = None;
                            let mut size_multiplier = None;
                            let mut trail = None;
                            let mut physics_scale = None;
                            let mut spawn_rate_factor = None;
                            let mut visible = None;
                            
                            // Parse remaining args as pairs of (param_name, value)
                            let mut i = 0;
                            while i + 1 < args.len() {
                                if let (osc::Type::String(param), value) = (&args[i], &args[i + 1]) {
                                    match param.as_str() {
                                        "alpha" => if let osc::Type::Float(val) = value { alpha = Some(*val); },
                                        "sizeMultiplier" => if let osc::Type::Float(val) = value { size_multiplier = Some(*val); },
                                        "trail" => if let osc::Type::Float(val) = value { trail = Some(*val); },
                                        "physicsScale" => if let osc::Type::Float(val) = value { physics_scale = Some(*val); },
                                        "spawnRateFactor" => if let osc::Type::Float(val) = value { spawn_rate_factor = Some(*val); },
                                        "visible" => if let osc::Type::Int(val) = value { visible = Some(*val != 0); },
                                        _ => {}
                                    }
                                }
                                i += 2;
                            }
                            
                            self.command_queue.push(OscCommand::ParticlesBatchUpdate {
                                id: *id,
                                alpha,
                                size_multiplier,
                                trail,
                                physics_scale,
                                spawn_rate_factor,
                                visible,
                            });
                        }
                    }
                    "/sys2/particles/debugEnable" => {
                        if let [osc::Type::Int(enabled)] = &message.args[..] {
                            self.command_queue.push(OscCommand::ParticlesDebugEnable {
                                enabled: *enabled != 0,
                            });
                        }
                    }
                    "/sys2/particles/debugToggle" => {
                        if let [osc::Type::String(feature), osc::Type::Int(enabled)] = &message.args[..] {
                            self.command_queue.push(OscCommand::ParticlesDebugToggle {
                                feature: feature.clone(),
                                enabled: *enabled != 0,
                            });
                        }
                    }
                    /********************* Drone Commands *************************** */
                    "/sys2/makeDrone" => {
                        if let [osc::Type::Int(id), osc::Type::Int(alpha), osc::Type::Int(num_particles), osc::Type::Int(force), osc::Type::Int(gravity), osc::Type::Int(trail)] =
                            &message.args[..]
                        {
                            self.command_queue.push(OscCommand::MakeDrone {
                                id: *id,
                                alpha: *alpha,
                                num_particles: *num_particles,
                                force: *force,
                                gravity: *gravity,
                                trail: *trail,
                            });
                        }
                    }
                    "/sys2/eraseDrone" => {
                        if let [osc::Type::Int(id)] = &message.args[..] {
                            self.command_queue.push(OscCommand::EraseDrone { id: *id });
                        }
                    }
                    /********************* Terminal Commands *************************** */
                    "/sys2/terminal/brightness" => {
                        if let [osc::Type::Int(id), osc::Type::Float(val)] = &message.args[..] {
                            self.command_queue
                                .push(OscCommand::TermBrightness { id: *id, val: *val });
                        }
                    }
                    "/sys2/terminal/volume" => {
                        if let [osc::Type::Int(id), osc::Type::Float(val)] = &message.args[..] {
                            self.command_queue
                                .push(OscCommand::TermVolume { id: *id, val: *val });
                        }
                    }
                    "/sys2/terminal/force" => {
                        if let [osc::Type::Int(id), osc::Type::Float(val)] = &message.args[..] {
                            self.command_queue
                                .push(OscCommand::TermForce { id: *id, val: *val });
                        }
                    }
                    "/sys2/terminal/shake" => {
                        if let [osc::Type::Int(id), osc::Type::Float(val)] = &message.args[..] {
                            self.command_queue
                                .push(OscCommand::TermShake { id: *id, val: *val });
                        }
                    }
                    "/sys2/terminal/feedback" => {
                        if let [osc::Type::Int(id), osc::Type::Float(val)] = &message.args[..] {
                            self.command_queue
                                .push(OscCommand::TermFeedback { id: *id, val: *val });
                        }
                    }
                    "/sys2/terminal/clear" => {
                        if let [osc::Type::Int(id)] = &message.args[..] {
                            self.command_queue.push(OscCommand::TermClear { id: *id });
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    pub fn take_commands(&mut self) -> Vec<OscCommand> {
        std::mem::take(&mut self.command_queue)
    }
}
