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
    ParticlesTrail {
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

    pub fn send_vibration(&self, player_id: i32, val: f32) {
        let addr = "/sys2/particles/vibration".to_string();
        let args = vec![osc::Type::Int(player_id), osc::Type::Float(val)];
        self.sender
            .send((addr, args), (self.target_addr.as_str(), self.target_port))
            .ok();
    }

    pub fn send_noise(&self, player_id: i32, val: f32) {
        let addr = "/sys2/particles/noise".to_string();
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
                            let val = val * 30.0;
                            self.command_queue
                                .push(OscCommand::ParticlesForce { id: *id, val });
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
                    "/sys2/particles/centerX" => {
                        if let [osc::Type::Int(id), osc::Type::Float(val)] = &message.args[..] {
                            self.command_queue
                                .push(OscCommand::ParticlesCenterX { id: *id, val: *val });
                        }
                    }
                    "/sys2/particles/centerY" => {
                        if let [osc::Type::Int(id), osc::Type::Float(val)] = &message.args[..] {
                            self.command_queue
                                .push(OscCommand::ParticlesCenterY { id: *id, val: *val });
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
                    "/sys2/particles/noise" => {
                        if let [osc::Type::Int(id), osc::Type::Float(val)] = &message.args[..] {
                            self.command_queue
                                .push(OscCommand::ParticlesNoise { id: *id, val: *val });
                        }
                    }
                    "/sys2/particles/vibration" => {
                        if let [osc::Type::Int(id), osc::Type::Float(val)] = &message.args[..] {
                            self.command_queue
                                .push(OscCommand::ParticlesVibration { id: *id, val: *val });
                        }
                    }
                    /********************* Mask Commands *************************** */
                    "/mask/create" => {
                        if let [osc::Type::Int(id), osc::Type::Float(x), osc::Type::Float(y), osc::Type::Float(w), osc::Type::Float(h)] =
                            &message.args[..]
                        {
                            self.command_queue.push(OscCommand::MaskCreate {
                                id: *id,
                                x: *x,
                                y: *y,
                                w: *w,
                                h: *h,
                                effect_preset: None,
                                layer: 0,
                            });
                        } else if let [osc::Type::Int(id), osc::Type::Float(x), osc::Type::Float(y), osc::Type::Float(w), osc::Type::Float(h), osc::Type::String(effect)] =
                            &message.args[..]
                        {
                            self.command_queue.push(OscCommand::MaskCreate {
                                id: *id,
                                x: *x,
                                y: *y,
                                w: *w,
                                h: *h,
                                effect_preset: if effect.is_empty() {
                                    None
                                } else {
                                    Some(effect.clone())
                                },
                                layer: 0,
                            });
                        } else if let [osc::Type::Int(id), osc::Type::Float(x), osc::Type::Float(y), osc::Type::Float(w), osc::Type::Float(h), osc::Type::String(effect), osc::Type::Int(layer)] =
                            &message.args[..]
                        {
                            self.command_queue.push(OscCommand::MaskCreate {
                                id: *id,
                                x: *x,
                                y: *y,
                                w: *w,
                                h: *h,
                                effect_preset: if effect.is_empty() {
                                    None
                                } else {
                                    Some(effect.clone())
                                },
                                layer: *layer,
                            });
                        }
                    }
                    "/mask/delete" => {
                        if let [osc::Type::Int(id)] = &message.args[..] {
                            self.command_queue.push(OscCommand::MaskDelete { id: *id });
                        }
                    }
                    "/mask/resize" => {
                        if let [osc::Type::Int(id), osc::Type::Float(x), osc::Type::Float(y), osc::Type::Float(w), osc::Type::Float(h)] =
                            &message.args[..]
                        {
                            self.command_queue.push(OscCommand::MaskResize {
                                id: *id,
                                x: *x,
                                y: *y,
                                w: *w,
                                h: *h,
                            });
                        }
                    }
                    "/mask/effect" => {
                        if let [osc::Type::Int(id)] = &message.args[..] {
                            self.command_queue.push(OscCommand::MaskSetEffect {
                                id: *id,
                                effect_preset: None,
                            });
                        } else if let [osc::Type::Int(id), osc::Type::String(effect)] =
                            &message.args[..]
                        {
                            self.command_queue.push(OscCommand::MaskSetEffect {
                                id: *id,
                                effect_preset: if effect.is_empty() {
                                    None
                                } else {
                                    Some(effect.clone())
                                },
                            });
                        }
                    }
                    "/mask/param" => {
                        if let [osc::Type::Int(id), osc::Type::String(param_name), osc::Type::Float(value)] =
                            &message.args[..]
                        {
                            self.command_queue.push(OscCommand::MaskSetEffectParam {
                                id: *id,
                                param_name: param_name.clone(),
                                value: *value,
                            });
                        }
                    }
                    "/mask/enable" => {
                        if let [osc::Type::Int(id), osc::Type::Int(enabled)] = &message.args[..] {
                            self.command_queue.push(OscCommand::MaskEnable {
                                id: *id,
                                enabled: *enabled != 0,
                            });
                        }
                    }
                    "/mask/layer" => {
                        if let [osc::Type::Int(id), osc::Type::Int(layer)] = &message.args[..] {
                            self.command_queue.push(OscCommand::MaskSetLayer {
                                id: *id,
                                layer: *layer,
                            });
                        }
                    }
                    "/sys2/mask/changeBounds" => {
                        if let [osc::Type::Int(id), osc::Type::Int(x), osc::Type::Int(y), osc::Type::Int(w), osc::Type::Int(h), osc::Type::Float(dur)] =
                            &message.args[..]
                        {
                            self.command_queue.push(OscCommand::MaskChangeBounds {
                                id: *id,
                                x: *x,
                                y: *y,
                                w: *w,
                                h: *h,
                                duration: *dur,
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
