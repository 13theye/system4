// src/osc/osc_module.rs
//
// OSC sender for Gameover2025

use nannou_osc as osc;
use std::error::Error;

use crate::config::OscSendConfig;

#[derive(Debug)]
pub enum OscCommand {
    Seq {
        id: i32,  // BoardInstance ID
        col: i32, // Col number of lit up square
        row: i32, // Row number of lit up square
    },
    LineClear {
        id: i32,     // BoardInstance ID
        number: i32, // number of lines cleared
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

    pub fn send_seq(&self, id_number: i32, col: i32, row: i32) {
        let addr = "/gameover".to_string();
        let args = vec![
            osc::Type::Int(id_number),
            osc::Type::String("sound".to_string()),
            osc::Type::Int(col),
            osc::Type::Int(row),
        ];
        self.sender
            .send((addr, args), (self.target_addr.as_str(), self.target_port))
            .ok();
    }

    pub fn send_line_clear(&self, id_number: i32, num_lines: i32) {
        let addr = "/gameover".to_string();
        let args = vec![
            osc::Type::Int(id_number),
            osc::Type::String("completed".to_string()),
            osc::Type::Int(num_lines),
        ];
        self.sender
            .send((addr, args), (self.target_addr.as_str(), self.target_port))
            .ok();
    }

    pub fn send_game_over(&self, id_number: i32, num_lines: i32) {
        let addr = "/gameover".to_string();
        let args = vec![
            osc::Type::Int(id_number),
            osc::Type::String("over".to_string()),
            osc::Type::Int(num_lines),
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
                    "/recorder/start" => {
                        self.command_queue.push(OscCommand::RecorderStart {});
                    }
                    "/recorder/stop" => {
                        self.command_queue.push(OscCommand::RecorderStop {});
                    }
                    "/grid/backbone_fade" => {
                        if let [osc::Type::String(name), osc::Type::Float(r), osc::Type::Float(g), osc::Type::Float(b), osc::Type::Float(a), osc::Type::Float(duration)] =
                            &message.args[..]
                        {
                            self.command_queue.push(OscCommand::GridBackboneFade {
                                name: name.clone(),
                                r: *r,
                                g: *g,
                                b: *b,
                                a: *a,
                                duration: *duration,
                            });
                        }
                    }
                    "/grid/backbone_stroke" => {
                        if let [osc::Type::String(name), osc::Type::Float(stroke_weight)] =
                            &message.args[..]
                        {
                            self.command_queue.push(OscCommand::GridBackboneStroke {
                                name: name.clone(),
                                stroke_weight: *stroke_weight,
                            });
                        }
                    }
                    "/grid/create" => {
                        if let [osc::Type::String(name), osc::Type::String(show), osc::Type::Float(x), osc::Type::Float(y), osc::Type::Float(rot)] =
                            &message.args[..]
                        {
                            self.command_queue.push(OscCommand::GridCreate {
                                name: name.clone(),
                                show: show.clone(),
                                position: (*x, *y),
                                rotation: *rot,
                            });
                        }
                    }
                    "/grid/move" => {
                        if let [osc::Type::String(name), osc::Type::Float(x), osc::Type::Float(y), osc::Type::Float(duration)] =
                            &message.args[..]
                        {
                            self.command_queue.push(OscCommand::GridMove {
                                name: name.clone(),
                                x: *x,
                                y: *y,
                                duration: *duration,
                            });
                        }
                    }
                    "/grid/rotate" => {
                        if let [osc::Type::String(name), osc::Type::Float(angle)] =
                            &message.args[..]
                        {
                            self.command_queue.push(OscCommand::GridRotate {
                                name: name.clone(),
                                angle: *angle,
                            });
                        }
                    }
                    "/grid/scale" => {
                        if let [osc::Type::String(name), osc::Type::Float(scale)] =
                            &message.args[..]
                        {
                            self.command_queue.push(OscCommand::GridScale {
                                name: name.clone(),
                                scale: *scale,
                            });
                        }
                    }
                    "/grid/slide" => {
                        if let [osc::Type::String(name), osc::Type::String(axis), osc::Type::Int(number), osc::Type::Float(position)] =
                            &message.args[..]
                        {
                            self.command_queue.push(OscCommand::GridSlide {
                                name: name.clone(),
                                axis: axis.clone(),
                                number: *number,
                                position: *position,
                            });
                        }
                    }
                    "/background/flash" => {
                        if let [osc::Type::Float(r), osc::Type::Float(g), osc::Type::Float(b), osc::Type::Float(duration)] =
                            &message.args[..]
                        {
                            self.command_queue.push(OscCommand::BackgroundFlash {
                                r: *r,
                                g: *g,
                                b: *b,
                                duration: *duration,
                            });
                        }
                    }
                    "/background/color_fade" => {
                        if let [osc::Type::Float(r), osc::Type::Float(g), osc::Type::Float(b), osc::Type::Float(duration)] =
                            &message.args[..]
                        {
                            self.command_queue.push(OscCommand::BackgroundColorFade {
                                r: *r,
                                g: *g,
                                b: *b,
                                duration: *duration,
                            });
                        }
                    }
                    "/grid/glyph" => {
                        if let [osc::Type::String(name), osc::Type::Int(index), osc::Type::Int(animation_type)] =
                            &message.args[..]
                        {
                            self.command_queue.push(OscCommand::GridGlyph {
                                grid_name: name.clone(),
                                glyph_index: *index as usize,
                                animation_type_msg: *animation_type,
                            });
                        }
                    }
                    "/grid/instantglyphcolor" => {
                        if let [osc::Type::String(name), osc::Type::Float(r), osc::Type::Float(g), osc::Type::Float(b), osc::Type::Float(a)] =
                            &message.args[..]
                        {
                            self.command_queue.push(OscCommand::GridInstantGlyphColor {
                                grid_name: name.clone(),
                                r: *r,
                                g: *g,
                                b: *b,
                                a: *a,
                            });
                        }
                    }
                    "/grid/nextglyph" => {
                        if let [osc::Type::String(name), osc::Type::Int(animation_type)] =
                            &message.args[..]
                        {
                            self.command_queue.push(OscCommand::GridNextGlyph {
                                grid_name: name.clone(),
                                animation_type_msg: *animation_type,
                            });
                        }
                    }
                    "/grid/nextglyphcolor" => {
                        if let [osc::Type::String(name), osc::Type::Float(r), osc::Type::Float(g), osc::Type::Float(b), osc::Type::Float(a)] =
                            &message.args[..]
                        {
                            self.command_queue.push(OscCommand::GridNextGlyphColor {
                                grid_name: name.clone(),
                                r: *r,
                                g: *g,
                                b: *b,
                                a: *a,
                            });
                        }
                    }
                    "/grid/noglyph" => {
                        if let [osc::Type::String(name), osc::Type::Int(animation_type)] =
                            &message.args[..]
                        {
                            self.command_queue.push(OscCommand::GridNoGlyph {
                                grid_name: name.clone(),
                                animation_type_msg: *animation_type,
                            });
                        }
                    }
                    "/grid/overwrite" => {
                        if let [osc::Type::String(name)] = &message.args[..] {
                            self.command_queue.push(OscCommand::GridOverwrite {
                                grid_name: name.clone(),
                            });
                        }
                    }
                    "/grid/transitiontrigger" => {
                        if let [osc::Type::String(name)] = &message.args[..] {
                            self.command_queue.push(OscCommand::GridTransitionTrigger {
                                grid_name: name.clone(),
                            });
                        }
                    }
                    "/grid/transitionauto" => {
                        if let [osc::Type::String(name)] = &message.args[..] {
                            self.command_queue.push(OscCommand::GridTransitionAuto {
                                grid_name: name.clone(),
                            });
                        }
                    }
                    "/grid/togglevisibility" => {
                        if let [osc::Type::String(name)] = &message.args[..] {
                            self.command_queue.push(OscCommand::GridToggleVisibility {
                                grid_name: name.clone(),
                            });
                        }
                    }
                    "/grid/setvisibility" => {
                        if let [osc::Type::String(name), osc::Type::Int(setting)] =
                            &message.args[..]
                        {
                            let setting_bool = *setting != 0;
                            self.command_queue.push(OscCommand::GridSetVisibility {
                                grid_name: name.clone(),
                                setting: setting_bool,
                            });
                        }
                    }
                    "/grid/togglecolorful" => {
                        if let [osc::Type::String(name)] = &message.args[..] {
                            self.command_queue.push(OscCommand::GridToggleColorful {
                                grid_name: name.clone(),
                            });
                        }
                    }
                    "/grid/setcolorful" => {
                        if let [osc::Type::String(name), osc::Type::Int(setting)] =
                            &message.args[..]
                        {
                            let setting_bool = *setting != 0;
                            self.command_queue.push(OscCommand::GridSetColorful {
                                grid_name: name.clone(),
                                setting: setting_bool,
                            });
                        }
                    }
                    "/grid/setpowereffect" => {
                        if let [osc::Type::String(name), osc::Type::Int(setting)] =
                            &message.args[..]
                        {
                            let setting_bool = *setting != 0;
                            self.command_queue.push(OscCommand::GridSetPowerEffect {
                                grid_name: name.clone(),
                                setting: setting_bool,
                            });
                        }
                    }
                    "/transition/update" => {
                        let mut grid_name = String::new();
                        let mut steps = None;
                        let mut frame_duration = None;
                        let mut wandering = None;
                        let mut density = None;

                        for (i, arg) in message.args.iter().enumerate() {
                            match (i, arg) {
                                (0, osc::Type::String(name)) => grid_name = name.clone(),
                                (1, osc::Type::Int(s)) => steps = Some(*s as usize),
                                (2, osc::Type::Float(f)) => frame_duration = Some(*f),
                                (3, osc::Type::Float(w)) => wandering = Some(*w),
                                (4, osc::Type::Float(d)) => density = Some(*d),
                                _ => (),
                            }
                        }

                        self.command_queue.push(OscCommand::TransitionUpdate {
                            grid_name,
                            steps,
                            frame_duration,
                            wandering,
                            density,
                        });
                    }
                    _ => println!("Unknown OSC address pattern: {}", message.addr),
                };
            }
        }
    }

    pub fn take_commands(&mut self) -> Vec<OscCommand> {
        std::mem::take(&mut self.command_queue)
    }
}
