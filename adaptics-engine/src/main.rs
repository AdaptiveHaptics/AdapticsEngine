use clap::Parser;
// ## !NOTE: do NOT use mod. try to use everything through the lib crate

/// Adaptics Engine CLI <https://github.com/AdaptiveHaptics/AdapticsEngine>
///
/// Allows realtime playback of haptic patterns being developed in the designer <https://adaptivehaptics.github.io/AdapticsDesigner/>, using a WebSocket connection
#[derive(Parser, Debug)]
#[command(author, version, long_about, verbatim_doc_comment)]
struct AdapticsEngineCliArgs {
    #[clap(short, long, default_value = "127.0.0.1:8037")]
    websocket_bind_addr: String,

    /// Uses a mock haptic device instead of a Ultraleap haptic device.
    /// This still requires the Ultraleap SDK (and DLLs to be available), but does not attempt to connect to a device.
    /// Mock device configuration is DEVICE_UPDATE_RATE=20000hz, CALLBACK_RATE=500hz
    #[clap(short='m', long)]
    use_mock_streaming: bool,

    /// Disables hosting the WebSocket server. Likely only useful for testing.
    #[clap(long)]
    no_network: bool,

    /// Disables attempts to connect to the Ultraleap tracking service (leap motion controller).
    /// Enable this if you want to run the engine on a machine without the Gemini SDK installed.
    #[clap(short='t', long)]
    no_tracking: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let cli_args = AdapticsEngineCliArgs::parse();

    adaptics_engine::run_threads_and_wait(
        cli_args.use_mock_streaming,
        if cli_args.no_network { None } else { Some(cli_args.websocket_bind_addr) },
        !cli_args.no_tracking,
    )
}
