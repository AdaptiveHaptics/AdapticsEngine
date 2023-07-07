use clap::Parser;
// ## !NOTE: do NOT use mod. try to use everything through the lib crate

/// Renders patterns from the web based mid air haptics designer tool, over a WebSocket connection
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct MAHServerArgs {
    #[clap(short, long, default_value = "127.0.0.1:8037")]
    websocket_bind_addr: String,

    #[clap(short='m', long)]
    use_mock_streaming: bool,

    #[clap(short, long)]
    no_network: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error + Send>> {
    let cli_args = MAHServerArgs::parse();

    adaptics_engine::run_threads_and_wait(
        cli_args.use_mock_streaming,
        if cli_args.no_network { None } else { Some(cli_args.websocket_bind_addr) },
        true,
    )
}
