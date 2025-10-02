// src/services/sequencer.rs
//
//

use crate::{
    config::OscSendConfig,
    groups::{RhythmParams, VoiceId},
    osc::OscSender,
};

use crossbeam_channel as channel;
use prat::clockservice::{BeatEvent, BeatSubdivision, ClockService, TickEvent};
use std::{
    collections::HashMap,
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};
use thread_priority::*;

/// A Euclidian-like sequencer that sends OSC beat messages based on an internal pattern.
pub struct Sequencer {
    id: VoiceId,
    params: RhythmParams,

    // Data callback channel
    data_tx: channel::Sender<usize>,
    data_rx: channel::Receiver<usize>,

    // State and Config
    state: SequencerState,
    debug: bool,

    // Business Logic State
    beat_count: usize,
    current_beat: Option<usize>,
}

#[allow(clippy::too_many_arguments)]
impl Sequencer {
    pub fn new(
        id: VoiceId,
        params: RhythmParams,
        data_tx: channel::Sender<usize>,
        data_rx: channel::Receiver<usize>,
        debug: bool,
    ) -> Self {
        let state = SequencerState {
            is_advancing: false,
            is_sending: false,
        };
        Self {
            id,
            params,
            data_tx,
            data_rx,
            state,
            beat_count: 0,
            current_beat: None,
            debug,
        }
    }

    /// Start the sequencer with the current column set to 0.
    pub fn start(&mut self) {
        if !self.state.is_advancing {
            self.state.is_advancing = true;
            self.state.is_sending = true;
            self.current_beat = Some(0);

            if self.debug {
                println!("Sequencer: Started sequencer {}", self.id);
            }
        } else if self.debug {
            println!(
                "Sequencer: Already started sequencer {}, ignoring command.",
                self.id
            );
        }
    }

    /// Stop the sequencer and reset the current column to None.
    pub fn stop(&mut self) {
        self.state.is_advancing = false;
        self.state.is_sending = false;
        self.current_beat = None;
    }

    /// Pause sending and advancing while maintaining the current column.
    pub fn pause(&mut self) {
        self.state.is_advancing = false;
        self.state.is_sending = false;
    }

    /// Resume sending and advancing while maintaining the current column.
    pub fn resume(&mut self) {
        self.state.is_advancing = true;
        self.state.is_sending = true;
    }

    /// Advance the sequencer, called when the subscribed BeatEvent is received.
    fn increment(&mut self) {
        // if there is no current beat, it means the sequencer
        // hasn't started or is not advancing, so do nothing.
        let Some(mut beat) = self.current_beat else {
            return;
        };

        self.beat_count += 1;

        beat += 1;

        // Wrap around
        if beat > self.params.capacity - 1 {
            beat = 0;
        }

        self.current_beat = Some(beat);

        // Send the beat to the data callback channel
        self.send_callback(beat);
    }

    /// Via callback channel, send the current beat
    fn send_callback(&self, beat: usize) {
        let _ = self.data_tx.try_send(beat).or_else(|_| {
            let _ = self.data_rx.try_recv(); // clear the old data from the channel
            self.data_tx.try_send(beat)
        });
    }

    /// Via OSC sender, send Voice_id, capacity, wings, and current beat
    fn send_commands(&self, osc_sender: &OscSender) {
        // Don't send if the self flag is false
        if !self.state.is_sending || self.current_beat.is_none() {
            return;
        }

        // Don't send if sequencer has never incremented.
        let Some(beat) = self.current_beat else {
            return;
        };

        // Don't send if the slot is missing
        let Some(slot) = self.params.slots.get(beat) else {
            return;
        };

        // Send 1 if the current beat is in the wings, 0 if not
        let on_off: i32 = if self.params.wings.contains(&beat) {
            1
        } else {
            0
        };

        osc_sender.send_rhythm(
            self.id.to_i32(),
            beat as i32,
            on_off,
            slot.velocity,
            slot.length,
            slot.cutoff,
        );
    }

    /// Retrieve the sequencer's subscribed-to subdivision
    pub fn subscribed_to_subdivision(&self) -> &BeatSubdivision {
        &self.params.subdivision
    }

    /// Update the sequencer's state.
    pub fn update_state(&mut self, state: SequencerState) {
        self.state = state;
    }

    /// Update the sequencer's rhythmic parameters
    pub fn update_params(&mut self, params: RhythmParams) {
        self.params = params;
    }

    /// Check if the sequencer is sending.
    pub fn is_sending(&self) -> bool {
        self.state.is_sending
    }

    /// Check if the sequencer is advancing.
    pub fn is_advancing(&self) -> bool {
        self.state.is_advancing
    }
}

/// Bridges communication between the main thread and sequencer(s).
pub struct SequencerService {
    // Thread
    #[allow(dead_code)]
    sequencer_thread_join: Option<JoinHandle<()>>,
    sequencer_states: HashMap<VoiceId, SequencerState>,

    // Command channel
    command_tx: channel::Sender<SequencerCommand>,

    // State channel
    state_rx: channel::Receiver<(VoiceId, SequencerState)>,

    // Data channels (return from Sequencers by ID)
    data_rxs: HashMap<VoiceId, channel::Receiver<usize>>,
    data_txs: HashMap<VoiceId, channel::Sender<usize>>,

    // Subscribed clock's channels
    #[allow(dead_code)]
    beat_rx: channel::Receiver<BeatEvent>,
    #[allow(dead_code)]
    tick_rx: channel::Receiver<TickEvent>,

    // Debug messages
    debug: bool,
}

impl SequencerService {
    /// Creates a new SequencerServiceBuilder with required clock and OSC config.
    pub fn with_clock_and_osc_config<'a>(
        clock: &'a ClockService,
        osc_config: &'a OscSendConfig,
    ) -> SequencerServiceBuilder<'a> {
        SequencerServiceBuilder::new(clock, osc_config)
    }

    /********************** State Management **********************/

    /// Provides a way to access the syncronize with thesequencer thread's state from the main thread.
    pub fn update(&mut self) {
        while let Ok((id, state)) = self.state_rx.try_recv() {
            self.sequencer_states.insert(id, state);
        }
    }

    /// Returns a reference to the most recent state of all sequencers.
    pub fn sequencer_states(&self) -> &HashMap<VoiceId, SequencerState> {
        &self.sequencer_states
    }

    // Send a command to the sequencer thread to pass changes from the UI
    pub fn update_sequencer_state(&mut self, id: VoiceId, state: SequencerState) {
        let _ = self
            .command_tx
            .send(SequencerCommand::UpdateState { id, state });
    }

    /********************** Add and Remove Sequencers ******************************/

    /// Add a new sequencer to the service.
    pub fn add_sequencer(&mut self, id: VoiceId, params: RhythmParams) {
        let (data_tx, data_rx) = channel::bounded(1);

        let result = self.command_tx.send(SequencerCommand::Add {
            id,
            params,
            data_tx: data_tx.clone(),
            data_rx: data_rx.clone(),
        });

        // Save the data channels
        self.data_txs.insert(id, data_tx);
        self.data_rxs.insert(id, data_rx);

        if self.debug {
            println!(
                "SequencerService: Sent Add command with result: {:?}",
                result
            );
        }
    }

    /// Remove a sequencer from the service.
    pub fn remove_sequencer(&mut self, id: VoiceId) {
        let result = self.command_tx.send(SequencerCommand::Remove { id });
        if self.debug {
            println!(
                "SequencerService: Sent Remove command with result: {:?}",
                result
            );
        }
    }

    /********************** Commands to Sequencers ******************************/

    /// Start all sequencers.
    pub fn start_all(&mut self) {
        let result = self.command_tx.send(SequencerCommand::StartAll);
        if self.debug {
            println!(
                "SequencerService: Sent StartAll command with result: {:?}",
                result
            );
        }
    }

    /// Update parameters for a specific sequencer.
    pub fn update_sequencer_params(&mut self, id: VoiceId, params: RhythmParams) {
        let result = self
            .command_tx
            .send(SequencerCommand::UpdateParams { id, params });
        if self.debug {
            println!(
                "SequencerService: Sent UpdateParams command for {:?} with result: {:?}",
                id, result
            );
        }
    }

    /// Pause a specific sequencer.
    pub fn pause_sequencer(&mut self, id: VoiceId) {
        let result = self.command_tx.send(SequencerCommand::Pause { id });
        if self.debug {
            println!(
                "SequencerService: Sent Pause command for {:?} with result: {:?}",
                id, result
            );
        }
    }

    /// Resume a specific sequencer.
    pub fn resume_sequencer(&mut self, id: VoiceId) {
        let result = self.command_tx.send(SequencerCommand::Resume { id });
        if self.debug {
            println!(
                "SequencerService: Sent Resume command for {:?} with result: {:?}",
                id, result
            );
        }
    }

    /********************* Sequencer Communication Wiring **************************/

    /// Get the data channel to receive from a sequencer.
    pub fn data_channel(&self, id: VoiceId) -> Option<channel::Receiver<usize>> {
        self.data_rxs.get(&id).cloned()
    }

    /// Get the command channel for sending to the sequencer service.
    pub fn command_channel(&self) -> channel::Sender<SequencerCommand> {
        self.command_tx.clone()
    }
}

/// The thread manages the sequencers for each board.
/// It receives commands and clock events and triggers individual sequencers to process the events.
pub struct SequencerThread {
    // Beat channel
    beat_rx: channel::Receiver<BeatEvent>,

    // Tick channel
    tick_rx: channel::Receiver<TickEvent>,

    // Command channels
    command_rx: channel::Receiver<SequencerCommand>,

    // State channels
    state_tx: channel::Sender<(VoiceId, SequencerState)>,
    state_rx: channel::Receiver<(VoiceId, SequencerState)>,

    // Sequencers
    sequencers: HashMap<VoiceId, Sequencer>,

    // OSC Sender
    osc_sender: OscSender,

    // Time
    last_tick_time: i64,
    next_tick_time: i64,

    // Synchronize on start by starting only on the next whole note
    is_sync_starting: bool,

    // Debug
    debug: bool,
}

impl SequencerThread {
    pub fn run(&mut self) {
        if self.debug {
            println!("SequencerManager: Starting thread loop");
        }

        let mut now;
        let mut last_tick_instant = None;
        let mut last_state_send = Instant::now();
        let mut latency;
        let mut sleep_duration;
        let mut expected_interval = 1000; // initialize at 1000µs before stabilizing at the clock's expected interval

        loop {
            while let Ok(command) = self.command_rx.try_recv() {
                if self.debug {
                    println!("SequencerThread: Received command: {:?}", command);
                }
                match command {
                    SequencerCommand::Add {
                        id,
                        params,
                        data_tx,
                        data_rx,
                    } => {
                        if self.debug {
                            println!("SequencerThread: Adding sequencer {}", id);
                        }
                        self.sequencers
                            .insert(id, Sequencer::new(id, params, data_tx, data_rx, self.debug));
                    }
                    SequencerCommand::Pause { id } => {
                        if let Some(sequencer) = self.sequencers.get_mut(&id) {
                            sequencer.pause();
                        }
                    }
                    SequencerCommand::PauseAll => {
                        for sequencer in self.sequencers.values_mut() {
                            sequencer.pause();
                        }
                    }
                    SequencerCommand::Remove { id } => {
                        self.sequencers.remove(&id);
                    }
                    SequencerCommand::Resume { id } => {
                        if let Some(sequencer) = self.sequencers.get_mut(&id) {
                            sequencer.resume();
                        }
                    }
                    SequencerCommand::ResumeAll => {
                        for sequencer in self.sequencers.values_mut() {
                            sequencer.resume();
                        }
                    }

                    SequencerCommand::Start { id } => {
                        if let Some(sequencer) = self.sequencers.get_mut(&id) {
                            sequencer.start();
                        }
                    }
                    SequencerCommand::StartAll => {
                        self.is_sync_starting = true;
                    }
                    SequencerCommand::Stop { id } => {
                        if let Some(sequencer) = self.sequencers.get_mut(&id) {
                            sequencer.stop();
                        }
                    }
                    SequencerCommand::StopAll => {
                        for sequencer in self.sequencers.values_mut() {
                            sequencer.stop();
                        }
                    }
                    SequencerCommand::UpdateParams { id, params } => {
                        if let Some(sequencer) = self.sequencers.get_mut(&id) {
                            sequencer.update_params(params);
                        }
                    }
                    SequencerCommand::UpdateState { id, state } => {
                        if let Some(sequencer) = self.sequencers.get_mut(&id) {
                            sequencer.update_state(state);
                        }
                    }
                    SequencerCommand::Shutdown => {
                        return;
                    }
                }
            }

            while let Ok(beat_event) = self.beat_rx.try_recv() {
                if self.is_sync_starting {
                    self.sync_start_all(&beat_event);
                }

                for sequencer in self.sequencers.values_mut() {
                    if beat_event
                        .subdivisions
                        .contains(sequencer.subscribed_to_subdivision())
                    {
                        if sequencer.is_advancing() {
                            sequencer.increment();
                        }
                        if sequencer.is_sending() {
                            sequencer.send_commands(&self.osc_sender);
                        }
                    }
                }
            }

            // Use ticks to sync the thread to the clock.
            while let Ok(tick_event) = self.tick_rx.try_recv() {
                self.last_tick_time = self.next_tick_time;
                self.next_tick_time = tick_event.next_tick_time;
                expected_interval = self.next_tick_time - self.last_tick_time;
                last_tick_instant = Some(tick_event.tick_instant);
            }

            now = Instant::now();

            if let Some(last_tick_instant) = last_tick_instant {
                latency = now.duration_since(last_tick_instant);
            } else {
                latency = Duration::from_micros(0);
            }

            // Send sequencer state to the main thread every frame.
            if now.duration_since(last_state_send) > Duration::from_micros(1_000_000 / 60) {
                for sequencer in self.sequencers.values() {
                    let _ = self
                        .state_tx
                        .try_send((sequencer.id, sequencer.state.clone()))
                        .or_else(|_| {
                            let _ = self.state_rx.try_recv(); // clear the old data from the channel
                            self.state_tx
                                .try_send((sequencer.id, sequencer.state.clone()))
                        });
                }
                last_state_send = now;
            }

            sleep_duration = self
                .next_tick_time
                .saturating_sub(self.last_tick_time)
                .saturating_sub(latency.as_micros() as i64);
            if self.next_tick_time == 0 || self.last_tick_time == 0 {
                // No valid timing info yet, sleep a short amount
                thread::sleep(Duration::from_millis(1));
            } else if sleep_duration > 1000 && sleep_duration < expected_interval {
                // Wake slightly early
                thread::sleep(Duration::from_micros(sleep_duration as u64 - 500));
            } else {
                std::hint::spin_loop();
            }
        }
    }

    pub fn sync_start_all(&mut self, beat_event: &BeatEvent) {
        if beat_event.subdivisions.contains(&BeatSubdivision::Whole) {
            for sequencer in self.sequencers.values_mut() {
                if self.debug {
                    println!("SequencerThread: Sync starting sequencer {}", sequencer.id);
                }
                sequencer.start();
            }
            self.is_sync_starting = false;
        } else if self.debug {
            println!("SequencerThread: Sync starting: waiting for next whole note");
        }
    }
}

/// Commands to the SequencerThread.
#[derive(Debug)]
pub enum SequencerCommand {
    Add {
        id: VoiceId,
        params: RhythmParams,
        data_tx: channel::Sender<usize>,
        data_rx: channel::Receiver<usize>,
    },

    Pause {
        id: VoiceId,
    },
    PauseAll,
    Resume {
        id: VoiceId,
    },
    ResumeAll,
    Remove {
        id: VoiceId,
    },
    Shutdown,
    Start {
        id: VoiceId,
    },
    StartAll,
    Stop {
        id: VoiceId,
    },
    StopAll,
    UpdateParams {
        id: VoiceId,
        params: RhythmParams,
    },
    UpdateState {
        id: VoiceId,
        state: SequencerState,
    },
}

#[derive(Clone, Debug)]
/// The state of a sequencer for an individual BoardInstance
pub struct SequencerState {
    pub is_advancing: bool,
    pub is_sending: bool,
}

/// Builder pattern for SequencerService.
/// .build() also starts the sequencer thread.
pub struct SequencerServiceBuilder<'a> {
    clock: &'a ClockService,
    osc_config: &'a OscSendConfig,
    thread_priority: u8,
    debug: bool,
}

impl<'a> SequencerServiceBuilder<'a> {
    pub fn new(clock: &'a ClockService, osc_config: &'a OscSendConfig) -> Self {
        Self {
            clock,
            osc_config,
            thread_priority: 31,
            debug: false,
        }
    }

    /// The Sequencer will be synchronized to this clock.
    pub fn clock(mut self, clock: &'a ClockService) -> Self {
        self.clock = clock;
        self
    }

    /// Set the thread priority. 31 is default on macOS. Highest allowed is 47.
    pub fn thread_priority(mut self, thread_priority: u8) -> Self {
        self.thread_priority = thread_priority;
        self
    }

    /// Set the debug flag.
    pub fn debug(mut self) -> Self {
        self.debug = true;
        self
    }

    /// Build and start the SequencerService.
    pub fn build(self) -> Option<SequencerService> {
        let Some(beat_rx) = self.clock.subscribe_to_beats() else {
            eprintln!("SequencerService: Clock service needs to be initialized before building SequencerService");
            return None;
        };
        let Some(tick_rx) = self.clock.subscribe_to_ticks() else {
            eprintln!("SequencerService: Clock service needs to be initialized before building SequencerService");
            return None;
        };
        let Ok(osc_sender) = OscSender::new(self.osc_config) else {
            eprintln!("SequencerService: OscSender failed to initialize");
            return None;
        };
        let (command_tx, command_rx) = channel::bounded(32);
        let (state_tx, state_rx) = channel::bounded(32);

        let mut sequencer_thread = SequencerThread {
            beat_rx: beat_rx.clone(),
            tick_rx: tick_rx.clone(),
            command_rx: command_rx.clone(),
            state_tx: state_tx.clone(),
            state_rx: state_rx.clone(),
            sequencers: HashMap::new(),
            osc_sender,
            last_tick_time: 0,
            next_tick_time: 0,
            is_sync_starting: false,
            debug: self.debug,
        };

        let sequencer_thread_handle = ThreadBuilder::default()
            .name("sequencer_thread".to_string())
            .priority(ThreadPriority::Crossplatform(
                ThreadPriorityValue::try_from(self.thread_priority).unwrap(),
            ))
            .spawn(move |result| {
                if self.debug {
                    println!("SequencerThread: Starting thread loop: {:?}", result);
                }
                assert!(result.is_ok());
                sequencer_thread.run();
            })
            .unwrap();

        let sequencer_thread_join = Some(sequencer_thread_handle);

        Some(SequencerService {
            sequencer_thread_join,
            sequencer_states: HashMap::new(),
            command_tx,
            state_rx,
            beat_rx,
            tick_rx,
            data_rxs: HashMap::new(),
            data_txs: HashMap::new(),
            debug: self.debug,
        })
    }
}
//                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             height,
//                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             data_tx,
//                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             data_rx,
//                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          osc_sender: OscSender,
