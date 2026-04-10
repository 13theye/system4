use prat::clockservice::ClockService;

use system4::{sequencer::SequencerService, settings::Settings};

pub fn init_clock_and_sequencer(settings: &Settings) -> (ClockService, SequencerService) {
    // Init clock
    let mut clock = ClockService::with()
        .tempo(settings.tempo.bpm as f64)
        .quantum(4.0)
        .ppqn(24)
        .enable_ticks()
        .thread_priority(47)
        .build();

    // Start the clock thread or quit game if it fails
    clock
        .start_thread()
        .expect("\nSystem4: fatal error: Failed to start clock thread");

    clock
        .start_clock()
        .expect("System4: fatal error: Failed to start clock");

    match clock.set_tempo(settings.tempo.bpm as f64) {
        Ok(_) => {
            println!("System4: Tempo set to {} bpm", settings.tempo.bpm);
        }
        Err(e) => eprintln!("System4: WARNING: Failed to assert tempo: {}", e),
    }

    let sequencer_service = SequencerService::with_clock_and_osc_config(&clock, &settings.osc_send)
        .build()
        .expect("System4: fatal error: Failed to build sequencer service");

    (clock, sequencer_service)
}
