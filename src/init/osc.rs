use system4::{
    osc::{OscController, OscSender},
    settings::{OscSendConfig, Settings},
};

pub fn init_osc(settings: &Settings) -> (OscController, OscSender, OscSender) {
    let osc = OscController::new(settings.osc_receive.receive_port)
        .expect("System4: fatal error: Failed to create OSC receiver");
    let osc_send = OscSender::new(&settings.osc_send)
        .expect("System4: fatal error: Failed to create OSC sender");

    let osc_loop_config = OscSendConfig {
        target_addr: settings.osc_loop.target_addr.clone(),
        target_port: settings.osc_loop.target_port,
    };
    let osc_loop = OscSender::new(&osc_loop_config)
        .expect("System4: fatal error: Failed to create OSC loop sender");

    (osc, osc_send, osc_loop)
}
