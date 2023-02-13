use std::net::TcpListener;
use std::thread;
use pattern_evaluator::PatternEvaluatorParameters;
use serde::{Serialize, Deserialize};
use tungstenite::accept;

use crate::PatternEvalCall;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "cmd", content = "data")]
#[serde(rename_all = "snake_case")]
enum PEWSClientMessage {
    UpdatePattern{ pattern_json: String },
    UpdatePlaystart{ playstart: f64, playstart_offset: f64 },
    UpdateParameters{ evaluator_params: PatternEvaluatorParameters },
}

pub enum PEWSServerMessage {
    PlaybackUpdate()
}


/// A WebSocket echo server
pub fn start_ws_server(patteval_call_tx: crossbeam_channel::Sender<PatternEvalCall>, network_send_rx: crossbeam_channel::Receiver<PEWSServerMessage>) {
    let server = TcpListener::bind("127.0.0.1:8080").unwrap();
    for stream in server.incoming() {
        let patteval_call_tx = patteval_call_tx.clone();
        thread::spawn(move || {
            let mut websocket = accept(stream.unwrap()).unwrap();
            loop {
                match websocket.read_message() {
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
                        break;
                    },
                    Err(err) => {
                        panic!("{:?}", err);
                    },
                }

            }
        });
    }
}
