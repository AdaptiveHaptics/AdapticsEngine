use std::thread;
use pattern_evaluator::BrushAtAnimLocalTime;

mod threads;
use threads::net::websocket;
use threads::pattern::pattern_eval;
use pattern_eval::PatternEvalUpdate;
use threads::streaming;
// use thread_priority;

const CALLBACK_RATE: f64 = 500.0;
const SECONDS_PER_NETWORK_SEND: f64 = 1.0 / 30.0;
const DEVICE_UPDATE_RATE: u64 = 20000; //20khz

use clap::Parser;

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


#[derive(Debug)]
struct TLError {
    details: String
}
impl TLError {
    fn new(msg: &str) -> TLError {
        TLError{ details: msg.to_string() }
    }
}
impl std::fmt::Display for TLError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f,"{}",self.details)
    }
}
impl std::error::Error for TLError {
    fn description(&self) -> &str {
        &self.details
    }
}

fn main() -> Result<(), Box<dyn std::error::Error + Send>> {
    let cli_args = MAHServerArgs::parse();

    let (patteval_call_tx, patteval_call_rx) = crossbeam_channel::unbounded();
    let (patteval_update_tx, patteval_update_rx) = crossbeam_channel::unbounded();
    let (patteval_return_tx, patteval_return_rx) = crossbeam_channel::bounded::<Vec<BrushAtAnimLocalTime>>(0);
    let (network_send_tx, network_send_rx) = crossbeam_channel::bounded(1);

    let (its_over_tx, its_over_rx) = crossbeam_channel::bounded(1);

    // thread_priority::set_current_thread_priority(thread_priority::ThreadPriority::Max).unwrap();

    let pattern_eval_handle = thread::Builder::new()
        .name("pattern-eval".to_string())
        .spawn(move || {
            println!("pattern-eval thread starting...");

            let res = pattern_eval::pattern_eval_loop(
                SECONDS_PER_NETWORK_SEND,
                patteval_call_rx,
                patteval_update_rx,
                patteval_return_tx,
                network_send_tx,
            );

            // res.unwrap();
            res.ok(); // ignore error, only occurs when channel disconnected

            println!("pattern-eval thread exiting...");
        })
        .unwrap();

    let ulh_streaming_handle_opt = if !cli_args.use_mock_streaming {
        Some(thread::Builder::new()
            .name("ulh-streaming".to_string())
            .spawn(move || -> Result<(), Box<dyn std::error::Error + Send>> {
                println!("ulhaptics streaming thread starting...");

                streaming::ulhaptics::start_streaming_emitter(
                    CALLBACK_RATE as f32,
                    patteval_call_tx,
                    patteval_return_rx,
                    its_over_rx,
                )
            }).unwrap())
    } else {
        println!("using mock streaming");
        Some(thread::Builder::new()
            .name("mock-streaming".to_string())
            .spawn(move || {
                println!("mock streaming thread starting...");

                streaming::mock::start_mock_emitter(
                    DEVICE_UPDATE_RATE,
                    CALLBACK_RATE,
                    patteval_call_tx,
                    patteval_return_rx,
                    its_over_rx,
                );

                // println!("mock streaming thread exiting...");
                Ok(())
            })
            .unwrap())
     };


    let net_handle_opt = if !cli_args.no_network {
        let thread = thread::Builder::new()
            .name("net".to_string())
            .spawn(move || {
                println!("net thread starting...");
                websocket::start_ws_server(&cli_args.websocket_bind_addr, patteval_update_tx, network_send_rx);
                println!("net thread thread exiting...");
            })
            .unwrap();
        Some(thread)
    } else { None };


    pattern_eval_handle.join().unwrap();
    its_over_tx.send(()).ok(); // ignore send error (if thread already exited)
    if let Some(h) = ulh_streaming_handle_opt { h.join().unwrap()?; } // unwrap panics, return errors
    println!("waiting for net thread...");
    if let Some(h) = net_handle_opt { h.join().unwrap() }

    Ok(())
}
