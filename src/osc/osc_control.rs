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
    ParticlesCenterX {
        id: i32,
        val: f32,
    },
    ParticlesCenterY {
        id: i32,
        val: f32,
    },
    ParticlesGravity {
        id: i32,
        val: f32,
    },
    ParticlesFeedback {
        id: i32,
        val: f32,
    },
    ParticlesNoise {
        id: i32,
        val: f32,
    },
    ParticlesVibration {
        id: i32,
        val: f32,
    },

    MaskChangeBounds {
        id: i32,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        duration: f32,
    },

    // New mask commands for RenderWindow integration
    MaskCreate {
        id: i32,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        effect_preset: Option<String>,
        layer: i32,
    },
    MaskDelete {
        id: i32,
    },
    MaskResize {
        id: i32,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
    },
    MaskSetEffect {
        id: i32,
        effect_preset: Option<String>,
    },
    MaskSetEffectParam {
        id: i32,
        param_name: String,
        value: f32,
    },
    MaskEnable {
        id: i32,
        enabled: bool,
    },
    MaskSetLayer {
        id: i32,
        layer: i32,
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
        let addr = "/sys4/droneOnOff".to_string();
        let args = vec![osc::Type::Int(player_id), osc::Type::Int(val)];
        self.sender
            .send((addr, args), (self.target_addr.as_str(), self.target_port))
            .ok();
    }

    pub fn send_inner_radius(&self, player_id: i32, val: f32) {
        let addr = "/sys4/particles/innerRadius".to_string();
        let args = vec![osc::Type::Int(player_id), osc::Type::Float(val)];
        self.sender
            .send((addr, args), (self.target_addr.as_str(), self.target_port))
            .ok();
    }

    pub fn send_outer_radius(&self, player_id: i32, val: f32) {
        let addr = "/sys4/particles/outerRadius".to_string();
        let args = vec![osc::Type::Int(player_id), osc::Type::Float(val)];
        self.sender
            .send((addr, args), (self.target_addr.as_str(), self.target_port))
            .ok();
    }

    pub fn send_vibration(&self, player_id: i32, val: f32) {
        let addr = "/sys4/particles/vibration".to_string();
        let args = vec![osc::Type::Int(player_id), osc::Type::Float(val)];
        self.sender
            .send((addr, args), (self.target_addr.as_str(), self.target_port))
            .ok();
    }

    pub fn send_noise(&self, player_id: i32, val: f32) {
        let addr = "/sys4/particles/noise".to_string();
        let args = vec![osc::Type::Int(player_id), osc::Type::Float(val)];
        self.sender
            .send((addr, args), (self.target_addr.as_str(), self.target_port))
            .ok();
    }

    pub fn send_mask_change_bounds(
        &self,
        player_id: i32,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        dur: f32,
    ) {
        let addr = "/sys2/mask/changeBounds".to_string();
        let args = vec![
            osc::Type::Int(player_id),
            osc::Type::Int(x),
            osc::Type::Int(y),
            osc::Type::Int(w),
            osc::Type::Int(h),
            osc::Type::Float(dur),
        ];
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
                    "/sys4/particles/alpha" => {
                        if let [osc::Type::Int(id), osc::Type::Float(val)] = &message.args[..] {
                            self.command_queue
                                .push(OscCommand::ParticlesAlpha { id: *id, val: *val });
                        }
                    }
                    "/sys4/particles/numParticles" => {
                        if let [osc::Type::Int(id), osc::Type::Float(val)] = &message.args[..] {
                            self.command_queue
                                .push(OscCommand::ParticlesNumParticles { id: *id, val: *val });
                        }
                    }
                    "/sys4/particles/force" => {
                        if let [osc::Type::Int(id), osc::Type::Float(val)] = &message.args[..] {
                            let val = val * 30.0;
                            self.command_queue
                                .push(OscCommand::ParticlesForce { id: *id, val });
                        }
                    }
                    "/sys4/particles/innerRadius" => {
                        if let [osc::Type::Int(id), osc::Type::Float(val)] = &message.args[..] {
                            self.command_queue
                                .push(OscCommand::ParticlesInnerRadius { id: *id, val: *val });
                        }
                    }
                    "/sys4/particles/outerRadius" => {
                        if let [osc::Type::Int(id), osc::Type::Float(val)] = &message.args[..] {
                            self.command_queue
                                .push(OscCommand::ParticlesOuterRadius { id: *id, val: *val });
                        }
                    }
                    "/sys2/particles/centerX" => {
                        if let [osc::Type::Int(id), osc::Type::Float(val)] = &message.args[..] {
                            self.command_queue
                                .push(OscCommand::ParticlesCenterX { id: *id, val: *val });
                        }
                    }
                    "/sys4/particles/centerY" => {
                        if let [osc::Type::Int(id), osc::Type::Float(val)] = &message.args[..] {
                            self.command_queue
                                .push(OscCommand::ParticlesCenterY { id: *id, val: *val });
                        }
                    }
                    "/sys4/particles/gravity" => {
                        if let [osc::Type::Int(id), osc::Type::Float(val)] = &message.args[..] {
                            self.command_queue
                                .push(OscCommand::ParticlesGravity { id: *id, val: *val });
                        }
                    }
                    "/sys4/particles/feedback" => {
                        if let [osc::Type::Int(id), osc::Type::Float(val)] = &message.args[..] {
                            self.command_queue
                                .push(OscCommand::ParticlesFeedback { id: *id, val: *val });
                        }
                    }
                    "/sys4/particles/noise" => {
                        if let [osc::Type::Int(id), osc::Type::Float(val)] = &message.args[..] {
                            self.command_queue
                                .push(OscCommand::ParticlesNoise { id: *id, val: *val });
                        }
                    }
                    "/sys4/particles/vibration" => {
                        if let [osc::Type::Int(id), osc::Type::Float(val)] = &message.args[..] {
                            self.command_queue
                                .push(OscCommand::ParticlesVibration { id: *id, val: *val });
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
