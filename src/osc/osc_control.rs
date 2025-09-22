// src/osc/osc_control.rs
//
// OSC commands, sender, and receiver for System3

use nannou_osc as osc;
use std::error::Error;

use crate::{
    config::OscSendConfig,
    groups::VoiceId,
    model::controller::{Command, CommandInner, CommandSource, SimpleCommand},
};

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
    receiver: osc::Receiver,
}

impl OscController {
    pub fn new(port: u16) -> Result<Self, Box<dyn Error>> {
        let receiver = osc::receiver(port)?;

        Ok(Self { receiver })
    }

    pub fn process_messages(&mut self) -> Vec<Command> {
        let mut commands = Vec::new();
        for (packet, _addr) in self.receiver.try_iter() {
            for message in packet.into_msgs() {
                match message.addr.as_str() {
                    /********************* Particle Commands *************************** */
                    "/sys4/particles/alpha" => {
                        if let [osc::Type::Int(id), osc::Type::Float(val)] = &message.args[..] {
                            let voice = VoiceId::from_i32(*id);
                            commands.push(Command::new(
                                CommandInner::Simple(SimpleCommand::Alpha {
                                    voice_id: voice,
                                    value: *val,
                                }),
                                CommandSource::Osc,
                            ));
                        }
                    }
                    "/sys4/particles/numParticles" => {
                        if let [osc::Type::Int(id), osc::Type::Float(val)] = &message.args[..] {
                            let voice = VoiceId::from_i32(*id);
                            commands.push(Command::new(
                                CommandInner::Simple(SimpleCommand::Volume {
                                    voice_id: voice,
                                    value: *val,
                                }),
                                CommandSource::Osc,
                            ));
                        }
                    }
                    "/sys4/particles/force" => {
                        if let [osc::Type::Int(id), osc::Type::Int(circle_id), osc::Type::Float(val)] =
                            &message.args[..]
                        {
                            let voice = VoiceId::from_i32(*id);
                            let val = val * 30.0;
                            commands.push(Command::new(
                                CommandInner::Simple(SimpleCommand::Force {
                                    voice_id: voice,
                                    circle_id: *circle_id as usize,
                                    value: val,
                                }),
                                CommandSource::Osc,
                            ));
                        }
                    }
                    "/sys4/particles/innerRadius" => {
                        if let [osc::Type::Int(id), osc::Type::Int(circle_id), osc::Type::Float(val)] =
                            &message.args[..]
                        {
                            let voice = VoiceId::from_i32(*id);
                            commands.push(Command::new(
                                CommandInner::Simple(SimpleCommand::InnerRadius {
                                    voice_id: voice,
                                    circle_id: *circle_id as usize,
                                    value: *val,
                                }),
                                CommandSource::Osc,
                            ));
                        }
                    }
                    "/sys4/particles/outerRadius" => {
                        if let [osc::Type::Int(id), osc::Type::Int(circle_id), osc::Type::Float(val)] =
                            &message.args[..]
                        {
                            let voice = VoiceId::from_i32(*id);
                            commands.push(Command::new(
                                CommandInner::Simple(SimpleCommand::OuterRadius {
                                    voice_id: voice,
                                    circle_id: *circle_id as usize,
                                    value: *val,
                                }),
                                CommandSource::Osc,
                            ));
                        }
                    }
                    "/sys2/particles/centerX" => {
                        if let [osc::Type::Int(id), osc::Type::Int(circle_id), osc::Type::Float(val)] =
                            &message.args[..]
                        {
                            let voice = VoiceId::from_i32(*id);
                            commands.push(Command::new(
                                CommandInner::Simple(SimpleCommand::CenterX {
                                    voice_id: voice,
                                    circle_id: *circle_id as usize,

                                    value: *val,
                                }),
                                CommandSource::Osc,
                            ));
                        }
                    }
                    "/sys4/particles/centerY" => {
                        if let [osc::Type::Int(id), osc::Type::Int(circle_id), osc::Type::Float(val)] =
                            &message.args[..]
                        {
                            let voice = VoiceId::from_i32(*id);
                            commands.push(Command::new(
                                CommandInner::Simple(SimpleCommand::CenterY {
                                    voice_id: voice,
                                    circle_id: *circle_id as usize,
                                    value: *val,
                                }),
                                CommandSource::Osc,
                            ));
                        }
                    }
                    "/sys4/particles/gravity" => {
                        if let [osc::Type::Int(id), osc::Type::Int(circle_id), osc::Type::Float(val)] =
                            &message.args[..]
                        {
                            let voice = VoiceId::from_i32(*id);
                            commands.push(Command::new(
                                CommandInner::Simple(SimpleCommand::Gravity {
                                    voice_id: voice,
                                    circle_id: *circle_id as usize,
                                    value: *val,
                                }),
                                CommandSource::Osc,
                            ));
                        }
                    }
                    "/sys4/particles/feedback" => {
                        if let [osc::Type::Int(id), osc::Type::Float(val)] = &message.args[..] {
                            let voice = VoiceId::from_i32(*id);
                            commands.push(Command::new(
                                CommandInner::Simple(SimpleCommand::Feedback {
                                    voice_id: voice,
                                    value: *val,
                                }),
                                CommandSource::Osc,
                            ));
                        }
                    }
                    "/sys4/particles/noise" => {
                        if let [osc::Type::Int(id), osc::Type::Int(circle_id), osc::Type::Float(val)] =
                            &message.args[..]
                        {
                            let voice = VoiceId::from_i32(*id);
                            commands.push(Command::new(
                                CommandInner::Simple(SimpleCommand::Noise {
                                    voice_id: voice,
                                    circle_id: *circle_id as usize,
                                    value: *val,
                                }),
                                CommandSource::Osc,
                            ));
                        }
                    }
                    "/sys4/particles/vibration" => {
                        if let [osc::Type::Int(id), osc::Type::Float(val)] = &message.args[..] {
                            let voice = VoiceId::from_i32(*id);
                            commands.push(Command::new(
                                CommandInner::Simple(SimpleCommand::Vibration {
                                    voice_id: voice,
                                    value: *val,
                                }),
                                CommandSource::Osc,
                            ));
                        }
                    }
                    _ => {}
                }
            }
        }
        commands
    }
}
