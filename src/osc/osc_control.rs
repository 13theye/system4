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
    pub fn send_drone_on_off(&self, voice_id: i32, val: i32) {
        let addr = "/sys4/droneOnOff".to_string();
        let args = vec![osc::Type::Int(voice_id), osc::Type::Int(val)];
        self.sender
            .send((addr, args), (self.target_addr.as_str(), self.target_port))
            .ok();
    }

    pub fn send_new_circle(&self, voice_id: i32, circle_id: i32) {
        let addr = "/sys4/newCircle".to_string();
        let args = vec![osc::Type::Int(voice_id), osc::Type::Int(circle_id)];
        self.sender
            .send((addr, args), (self.target_addr.as_str(), self.target_port))
            .ok();
    }

    pub fn send_end_circle(&self, voice_id: i32, circle_id: i32) {
        let addr = "/sys4/endCircle".to_string();
        let args = vec![osc::Type::Int(voice_id), osc::Type::Int(circle_id)];
        self.sender
            .send((addr, args), (self.target_addr.as_str(), self.target_port))
            .ok();
    }

    pub fn send_inner_radius(&self, player_id: i32, val: f32) {
        let addr = "/sys4/circle/innerRadius".to_string();
        let args = vec![osc::Type::Int(player_id), osc::Type::Float(val)];
        self.sender
            .send((addr, args), (self.target_addr.as_str(), self.target_port))
            .ok();
    }

    pub fn send_outer_radius(&self, player_id: i32, val: f32) {
        let addr = "/sys4/circle/outerRadius".to_string();
        let args = vec![osc::Type::Int(player_id), osc::Type::Float(val)];
        self.sender
            .send((addr, args), (self.target_addr.as_str(), self.target_port))
            .ok();
    }

    pub fn send_vibration(&self, player_id: i32, val: f32) {
        let addr = "/sys4/voice/vibration".to_string();
        let args = vec![osc::Type::Int(player_id), osc::Type::Float(val)];
        self.sender
            .send((addr, args), (self.target_addr.as_str(), self.target_port))
            .ok();
    }

    pub fn send_noise(&self, player_id: i32, val: f32) {
        let addr = "/sys4/circle/noise".to_string();
        let args = vec![osc::Type::Int(player_id), osc::Type::Float(val)];
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
                    "/sys4/voice/alpha" => {
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
                    "/sys4/voice/numParticles" => {
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
                    "/sys4/circle/force" => {
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
                    "/sys4/circle/innerRadius" => {
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
                    "/sys4/circle/outerRadius" => {
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
                    "/sys2/circle/centerX" => {
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
                    "/sys4/circle/centerY" => {
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
                    "/sys4/circle/gravity" => {
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
                    "/sys4/voice/feedback" => {
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
                    "/sys4/circle/noise" => {
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
                    "/sys4/voice/vibration" => {
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
