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
    ParticlesDeviation {
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
    pub fn send_drone_on_off(&self, player_id: usize, val: i32) {
        let addr = "/sys2/droneOnOff".to_string();
        let args = vec![osc::Type::Int(player_id as i32), osc::Type::Int(val)];
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
                    "/sys2/particles/deviation" => {
                        if let [osc::Type::Int(id), osc::Type::Float(val)] = &message.args[..] {
                            self.command_queue
                                .push(OscCommand::ParticlesDeviation { id: *id, val: *val });
                        }
                    }
                    "/sys2/particles/trail" => {
                        if let [osc::Type::Int(id), osc::Type::Float(val)] = &message.args[..] {
                            self.command_queue
                                .push(OscCommand::ParticlesGravity { id: *id, val: *val });
                        }
                    }
                    "/sys2/particles/gravity" => {
                        if let [osc::Type::Int(id), osc::Type::Float(val)] = &message.args[..] {
                            self.command_queue
                                .push(OscCommand::ParticlesTrail { id: *id, val: *val });
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
