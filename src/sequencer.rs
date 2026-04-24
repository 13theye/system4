// src/sequencer/mod.rs
//
//

use crate::{
    groups::{RhythmParams, VoiceId},
    osc::OscSender,
    settings::OscSendConfig,
};

use prat::clockservice::{BeatEvent, BeatSubdivision, ClockService, TickEvent};
use std::{
    collections::HashMap,
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};
use tokio::sync::broadcast;

/// A Euclidian-like sequencer that sends OSC beat messages based on an internal pattern.
pub struct Sequencer {
    id: VoiceId,
    params: RhythmParams,

    // Data callback channel
    data_tx: broadcast::Sender<usize>,
    data_rx: broadcast::Receiver<usize>,

    // State and Config
    state: SequencerState,
    debug: bool,

    // Business Logic State
    beat_count: usize,
    next_beat: Option<usize>,
}

impl Sequencer {
    pub fn new(
        id: VoiceId,
        params: RhythmParams,
        data_tx: broadcast::Sender<usize>,
        debug: bool,
    ) -> Self {
        let state = SequencerState {
            is_advancing: false,
            is_sending: false,
        };
        let data_rx = data_tx.subscribe();
        Self {
            id,
            params,
            data_tx,
            data_rx,
            state,
            beat_count: 0,
            next_beat: None,
            debug,
        }
    }

    /// Start the sequencer with the current column set to 0.
    pub fn start(&mut self) {
        self.state.is_advancing = true;
        self.state.is_sending = true;
        self.next_beat = Some(0);

        if self.debug {
            println!("Sequencer: Started sequencer {}", self.id);
        }
    }

    /// Stop the sequencer and reset the current column to None.
    pub fn stop(&mut self) {
        self.state.is_advancing = false;
        self.state.is_sending = false;
        self.next_beat = None;
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
        // hasn't started yet, so do nothing
        let Some(mut beat) = self.next_beat else {
            return;
        };

        // Advance the beat
        self.beat_count += 1;
        beat += 1;

        // Wrap around if necessary
        if beat > self.params.capacity - 1 {
            beat = 0;
        }

        // Update next beat
        self.next_beat = Some(beat);
    }

    /// Via callback channel, send the current beat
    fn send_callback(&mut self, beat: usize) {
        let _ = self.data_tx.send(beat).or_else(|_| {
            let _ = self.data_rx.try_recv(); // clear the old data from the channel
            self.data_tx.send(beat)
        });
    }

    /// Via OSC sender, send Voice_id, capacity, wings, and current beat
    fn send_osc(&self, osc_sender: &OscSender) {
        // Don't send if the self flag is false
        if !self.state.is_sending || self.next_beat.is_none() {
            return;
        }

        // Don't send if sequencer has never incremented.
        let Some(beat) = self.next_beat else {
            return;
        };

        // Don't send if the slot is missing
        let Some(slot) = self.params.slot_params.get(beat) else {
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
    command_tx: broadcast::Sender<SequencerCommand>,

    // State channel
    state_rx: broadcast::Receiver<(VoiceId, SequencerState)>,

    // Data channels (return from Sequencers by ID)
    data_rxs: HashMap<VoiceId, broadcast::Receiver<usize>>,
    data_txs: HashMap<VoiceId, broadcast::Sender<usize>>,

    // Subscribed clock's channels
    #[allow(dead_code)]
    beat_rx: broadcast::Receiver<BeatEvent>,
    #[allow(dead_code)]
    tick_rx: broadcast::Receiver<TickEvent>,

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
        let (data_tx, data_rx) = broadcast::channel(1);

        let result = self.command_tx.send(SequencerCommand::Add {
            id,
            params,
            data_tx: data_tx.clone(),
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

    /// Start a specific sequencer immediately (without whole-note resync).
    pub fn start_sequencer(&mut self, id: VoiceId) {
        let result = self.command_tx.send(SequencerCommand::Start { id });
        if self.debug {
            println!(
                "SequencerService: Sent Start command for {:?} with result: {:?}",
                id, result
            );
        }
    }

    /// Stop a specific sequencer immediately.
    pub fn stop_sequencer(&mut self, id: VoiceId) {
        let result = self.command_tx.send(SequencerCommand::Stop { id });
        if self.debug {
            println!(
                "SequencerService: Sent Stop command for {:?} with result: {:?}",
                id, result
            );
        }
    }

    /// Schedule a sequencer to start on the next whole-note boundary when a
    /// reference voice is at position 0. This keeps time and sequence aligned
    /// without restarting the reference voice.
    pub fn sync_start_to_voice(&mut self, id: VoiceId, reference: VoiceId) {
        let result = self
            .command_tx
            .send(SequencerCommand::SyncStartToVoice { id, reference });
        if self.debug {
            println!(
                "SequencerService: Sent SyncStartToVoice command for {:?} (ref {:?}) with result: {:?}",
                id, reference, result
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
    pub fn get_data_rx(&self, id: VoiceId) -> Option<broadcast::Receiver<usize>> {
        self.data_txs.get(&id).map(|tx| tx.subscribe())
    }

    /// Get the command channel for sending to the sequencer service.
    pub fn get_command_tx(&self) -> broadcast::Sender<SequencerCommand> {
        self.command_tx.clone()
    }
}

/// The thread manages the sequencers for each board.
/// It receives commands and clock events and triggers individual sequencers to process the events.
pub struct SequencerThread {
    // Beat channel
    beat_rx: broadcast::Receiver<BeatEvent>,

    // Tick channel
    tick_rx: broadcast::Receiver<TickEvent>,

    // Command channels
    command_rx: broadcast::Receiver<SequencerCommand>,

    // State channels
    state_tx: broadcast::Sender<(VoiceId, SequencerState)>,
    state_rx: broadcast::Receiver<(VoiceId, SequencerState)>,

    // Sequencers
    sequencers: HashMap<VoiceId, Sequencer>,

    // OSC Sender
    osc_sender: OscSender,

    // Time
    last_tick_time: i64,
    next_tick_time: i64,

    // Synchronize on start by starting only on the next whole note
    is_sync_starting: bool,

    // Pending request to sync-start a single sequencer relative to a reference voice
    // (target_id, reference_id)
    pending_sync_start: Option<(VoiceId, VoiceId)>,

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
                    } => {
                        if self.debug {
                            println!("SequencerThread: Adding sequencer {}", id);
                        }
                        self.sequencers
                            .insert(id, Sequencer::new(id, params, data_tx, self.debug));
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
                    SequencerCommand::SyncStartToVoice { id, reference } => {
                        // Defer actual start until beat handling where we can
                        // see both the clock (whole-note boundary) and the
                        // reference voice's position.
                        self.pending_sync_start = Some((id, reference));
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

                // Handle pending single-voice sync start: wait for a whole-note
                // event where the reference voice is at position 0, then start
                // only the target voice.
                if let Some((target_id, reference_id)) = self.pending_sync_start {
                    // Check the reference voice's next_beat without taking
                    // a mutable borrow yet.
                    let reference_at_zero = self
                        .sequencers
                        .get(&reference_id)
                        .map(|s| s.next_beat == Some(0))
                        .unwrap_or(false);

                    if reference_at_zero {
                        if let Some(target) = self.sequencers.get_mut(&target_id) {
                            if self.debug {
                                println!(
                                        "SequencerThread: Sync starting target {} relative to reference {}",
                                        target_id, reference_id
                                    );
                            }
                            target.start();
                            self.pending_sync_start = None;
                        } else if self.debug {
                            // Target was removed before we could start it; drop the request.
                            println!(
                                    "SequencerThread: SyncStartToVoice target {:?} missing; clearing request",
                                    target_id
                                );
                            self.pending_sync_start = None;
                        }
                    }
                }

                for sequencer in self.sequencers.values_mut() {
                    if beat_event
                        .subdivisions
                        .contains(sequencer.subscribed_to_subdivision())
                    {
                        if sequencer.is_sending() {
                            // Send OSC commands
                            sequencer.send_osc(&self.osc_sender);

                            // Send current beat to callback
                            let Some(beat) = sequencer.next_beat else {
                                continue;
                            };
                            sequencer.send_callback(beat);
                        }

                        // Advance the beat counts and next beat
                        if sequencer.is_advancing() {
                            sequencer.increment();
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
                        .send((sequencer.id, sequencer.state.clone()))
                        .or_else(|_| {
                            let _ = self.state_rx.try_recv(); // clear the old data from the channel
                            self.state_tx.send((sequencer.id, sequencer.state.clone()))
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
#[derive(Clone, Debug)]
pub enum SequencerCommand {
    Add {
        id: VoiceId,
        params: RhythmParams,
        data_tx: broadcast::Sender<usize>,
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
    /// Schedule a single sequencer to start on the next whole-note boundary
    /// when a reference voice is at position 0.
    SyncStartToVoice {
        id: VoiceId,
        reference: VoiceId,
    },
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
    debug: bool,
}

impl<'a> SequencerServiceBuilder<'a> {
    pub fn new(clock: &'a ClockService, osc_config: &'a OscSendConfig) -> Self {
        Self {
            clock,
            osc_config,
            debug: false,
        }
    }

    /// The Sequencer will be synchronized to this clock.
    pub fn clock(mut self, clock: &'a ClockService) -> Self {
        self.clock = clock;
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
        let (command_tx, _) = broadcast::channel(32);
        let (state_tx, state_rx) = broadcast::channel(32);

        let mut sequencer_thread = SequencerThread {
            beat_rx,
            tick_rx,
            command_rx: command_tx.subscribe(),
            state_tx: state_tx.clone(),
            state_rx: state_tx.subscribe(),
            sequencers: HashMap::new(),
            osc_sender,
            last_tick_time: 0,
            next_tick_time: 0,
            is_sync_starting: false,
            pending_sync_start: None,
            debug: self.debug,
        };

        let debug = self.debug;
        let sequencer_thread_handle = std::thread::Builder::new()
            .name("sequencer_thread".to_string())
            .spawn(move || {
                let _rt_handle =
                    audio_thread_priority::promote_current_thread_to_real_time(256, 48000)
                        .map_err(|e| eprintln!("SequencerThread: real-time promotion failed: {e}"))
                        .ok();
                if debug {
                    println!("SequencerThread: started (RT: {})", _rt_handle.is_some());
                }
                sequencer_thread.run();
            })
            .unwrap();

        let sequencer_thread_join = Some(sequencer_thread_handle);

        let Some(beat_rx) = self.clock.subscribe_to_beats() else {
            eprintln!("SequencerService: Clock service needs to be initialized before building SequencerService");
            return None;
        };
        let Some(tick_rx) = self.clock.subscribe_to_ticks() else {
            eprintln!("SequencerService: Clock service needs to be initialized before building SequencerService");
            return None;
        };

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
