use std::io::Write;
use std::time::Instant;

use futures_util::stream::SplitSink;
use pattern_evaluator::BrushAtAnimLocalTime;
use pattern_evaluator::PatternEvaluatorParameters;
use serde::{Serialize, Deserialize};

use tokio::net::TcpListener;
use futures_util::{SinkExt, StreamExt};
use tokio::sync::broadcast;
use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::tungstenite;

use crate::PatternEvalCall;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "cmd", content = "data")]
#[serde(rename_all = "snake_case")]
enum PEWSClientMessage {
    UpdatePattern{ pattern_json: String },
    UpdatePlaystart{ playstart: f64, playstart_offset: f64 },
    UpdateParameters{ evaluator_params: PatternEvaluatorParameters },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "cmd", content = "data")]
#[serde(rename_all = "snake_case")]
pub enum PEWSServerMessage {
    PlaybackUpdate{ evals: Vec<BrushAtAnimLocalTime> }
}


/// A WebSocket echo server
pub fn start_ws_server(patteval_call_tx: crossbeam_channel::Sender<PatternEvalCall>, mut network_send_rx: crossbeam_channel::Receiver<PEWSServerMessage>) {
    let rt  = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();

    rt.block_on(async {
        let server = TcpListener::bind("127.0.0.1:8037").await.unwrap();

        let (wstx_send, mut wstx_recv) = tokio::sync::mpsc::unbounded_channel();
        { // spawn a task to send messages to all connected clients
            let mut ws_tx_vec: Vec<SplitSink<WebSocketStream<_>, tungstenite::Message>> = Vec::new();
            tokio::task::spawn(async move {
                loop {
                    let network_send_rx = network_send_rx.clone();
                    println!("waiting for message...");
                    let msg = tokio::task::spawn_blocking(move || network_send_rx.recv()).await.unwrap().unwrap(); // the loop must finish in 1/60th of a second
                    // let msg = network_send_rx.recv().await.unwrap();
                    println!("got message...");
                    let begin_time = Instant::now();

                    loop {
                        match wstx_recv.try_recv() {
                            Err(tokio::sync::mpsc::error::TryRecvError::Empty) => { break; },
                            recv => { ws_tx_vec.push(recv.unwrap()); }
                        }
                    }

                    let mut i = 0;
                    while i < ws_tx_vec.len() {
                        if let Err(e) = ws_tx_vec[i].send(tungstenite::Message::Text(serde_json::to_string(&msg).unwrap())).await {
                            println!("[info] error sending to client: {}", e);
                            drop(ws_tx_vec.remove(i));
                        } else {
                            i += 1;
                        }
                    }
                    println!("sent message to {} clients in {:?}", ws_tx_vec.len(), begin_time.elapsed());
                }
            });
        }

        loop {
            match server.accept().await {
                Ok((stream, addr)) => {
                    println!("new connection from {}", addr);
                    let (ws_sender, mut ws_receiver) = tokio_tungstenite::accept_async(stream).await.unwrap().split();
                    wstx_send.send(ws_sender).unwrap();
                    let patteval_call_tx = patteval_call_tx.clone();
                    tokio::spawn(async move {
                        while let Some(recv) = ws_receiver.next().await {
                            match recv {
                                Ok(msg) => {
                                    match msg {
                                        tungstenite::Message::Text(data) => {
                                            let msg: PEWSClientMessage = serde_json::from_str(&data).unwrap();
                                            println!("received: {:?}", msg);
                                            match msg {
                                                PEWSClientMessage::UpdatePattern{ pattern_json } => {
                                                    patteval_call_tx.send(PatternEvalCall::UpdatePattern{ pattern_json }).unwrap();
                                                },
                                                PEWSClientMessage::UpdatePlaystart{ playstart, playstart_offset } => {
                                                    patteval_call_tx.send(PatternEvalCall::UpdatePlaystart{ playstart, playstart_offset }).unwrap();
                                                },
                                                PEWSClientMessage::UpdateParameters{ evaluator_params } => {
                                                    patteval_call_tx.send(PatternEvalCall::UpdateParameters{ evaluator_params }).unwrap();
                                                },
                                            }
                                        },
                                        tungstenite::Message::Binary(_) => todo!(),

                                        _ => (),

                                        // can ignore these
                                        // tungstenite::Message::Pong(_) => todo!(),
                                        // tungstenite::Message::Frame(_) => todo!(),

                                        // websocket.read_message() should queue responses to ping and close messages
                                        // tungstenite::Message::Ping(d) => websocket.write_message(tungstenite::Message::Pong(d)).unwrap(),
                                        // tungstenite::Message::Close(cf) => websocket.close(cf).unwrap(),
                                    }
                                },
                                Err(tungstenite::Error::ConnectionClosed) => {
                                    println!("websocket closed");
                                },
                                Err(err) => {
                                    panic!("{:?}", err);
                                },
                            }
                        }
                    });
                },
                Err(e) => {
                    println!("accept error = {:?}", e);
                }
            }
        }
    });

}