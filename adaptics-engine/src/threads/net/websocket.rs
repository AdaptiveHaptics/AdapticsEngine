use std::{io::prelude::*, sync::{Arc, Mutex}, time::Duration, net::TcpListener};
use std::{io::BufReader, net::TcpStream};
use pattern_evaluator::BrushAtAnimLocalTime;
use schemars::JsonSchema;
use serde::{Serialize, Deserialize};
use sha1::{Sha1, Digest};
use base64::{self, Engine as _};

use crate::{PatternEvalUpdate, threads::tracking};

/// Messages sent to websocket clients
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "cmd", content = "data")]
#[serde(rename_all = "snake_case")]
pub enum AdapticsWSServerMessage {
    /// Updates from the pattern evaluator to be sent to websocket/non-hardware clients
    PlaybackUpdate{ evals: Vec<BrushAtAnimLocalTime> },
    /// Updates from the tracking system to be sent to websocket clients
    TrackingData{ tracking_frame: tracking::TrackingFrame },
}

pub(crate) struct MAHWebsocket {
    bufread: BufReader<TcpStream>,
    uid: u64,
    _wsrecvjh: std::thread::JoinHandle<()>
}
impl MAHWebsocket {
    pub fn send(&mut self, wsm: &AdapticsWSServerMessage) -> std::io::Result<usize> {
        let payload = serde_json::to_string(wsm).unwrap();
        self.bufread.get_mut().write(&create_ws_frame(WsFrameOpcodes::Text, payload.as_bytes()))
    }
}

#[derive(Debug, Clone, Copy)]
enum WsFrameOpcodes {
    Continuation = 0x0,
    Text = 0x1,
    Binary = 0x2,
    Close = 0x8,
    Ping = 0x9,
    Pong = 0xA,
}
impl std::convert::TryFrom<u8> for WsFrameOpcodes {
    type Error = ();
    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            x if x == WsFrameOpcodes::Continuation as u8 => Ok(WsFrameOpcodes::Continuation),
            x if x == WsFrameOpcodes::Text as u8 => Ok(WsFrameOpcodes::Text),
            x if x == WsFrameOpcodes::Binary as u8 => Ok(WsFrameOpcodes::Binary),
            x if x == WsFrameOpcodes::Close as u8 => Ok(WsFrameOpcodes::Close),
            x if x == WsFrameOpcodes::Ping as u8 => Ok(WsFrameOpcodes::Ping),
            x if x == WsFrameOpcodes::Pong as u8 => Ok(WsFrameOpcodes::Pong),
            _ => Err(()),
        }
    }
}

struct WsFrameRecvd {
    fin: bool,
    opcode: WsFrameOpcodes,
    payload: Vec<u8>
}
fn create_ws_frame(opcode: WsFrameOpcodes, payload: &[u8]) -> Vec<u8> {
    // println!("{:#?}=={:#b}, {:#?}", opcode, opcode as u8, payload.len());
    let payloadlen = payload.len();
    let mut frame: Vec<u8> = Vec::with_capacity(16+payloadlen);
    frame.push(0b1000_0000 | (opcode as u8)); //set FIN=0 RSV=0 opcode
    #[allow(clippy::cast_possible_truncation)]
    { //payload length
        match payloadlen { //MASK is always false
            0..=125 => {
                frame.push(payloadlen as u8);
            }
            126..=65535 => {
                frame.push(0b0111_1110);
                frame.push((payloadlen>>8) as u8);
                frame.push(payloadlen as u8);
            }
            65536..=9_223_372_036_854_775_807 => {
                frame.push(0b0111_1111);
                frame.push((payloadlen>>56) as u8);
                frame.push((payloadlen>>48) as u8);
                frame.push((payloadlen>>40) as u8);
                frame.push((payloadlen>>32) as u8);
                frame.push((payloadlen>>24) as u8);
                frame.push((payloadlen>>16) as u8);
                frame.push((payloadlen>>8) as u8);
                frame.push(payloadlen as u8);
            }
            _ => unreachable!()
        }
    }
    frame.extend_from_slice(payload);
    // println!("{:?}", frame);
    frame
}
/// returns (`fin` (see rfc6455#section-5.2), `opcode`, `payload_length`, `payload_start_index`)
fn parse_ws_frame_header(frame: &[u8]) -> Option<(bool, WsFrameOpcodes, usize, usize, [u8; 4])> {
    let mut i_aii = 0;
    let framelen = frame.len();
    let mut aii = || {let oi=i_aii; i_aii+=1; if oi<framelen {Some(oi)} else {None} };

    let frameb0 = frame[aii()?];
    let fin = (frameb0 & 0b1000_0000) > 0;
    let opcode = (frameb0 & 0b0000_1111).try_into().unwrap();
    let rawlen = frame[aii()?] & 0b0111_1111;
    let payload_length = match rawlen {
        0..=125 => { (rawlen).into() } // TODO: close ws with error instead of panic
        126 => {
            (u16::from(frame[aii()?])<<8 | u16::from(frame[aii()?])).into()
        }
        127 => {
            (
                u64::from(frame[aii()?])<<56 |
                u64::from(frame[aii()?])<<48 |
                u64::from(frame[aii()?])<<40 |
                u64::from(frame[aii()?])<<32 |
                u64::from(frame[aii()?])<<24 |
                u64::from(frame[aii()?])<<16 |
                u64::from(frame[aii()?])<<8  |
                u64::from(frame[aii()?])
            ).try_into().unwrap() // TODO: close ws with error instead of panic
        }
        _ => unreachable!()
    };
    let masking_key: [u8; 4] = [frame[aii()?], frame[aii()?], frame[aii()?], frame[aii()?]];
    Some((fin, opcode, payload_length, i_aii, masking_key))
}
fn parse_ws_frame_body(frame: &[u8], payload_length: usize, payload_start_index: usize, masking_key: [u8; 4]) -> Option<(usize, Vec<u8>)> {
    let bytes_read = payload_length+payload_start_index;
    let payloadmasked = &frame[payload_start_index..bytes_read];
    let mut payload = Vec::with_capacity(payload_length);
    for i in 0..payload_length {
        payload.push(payloadmasked.get(i)? ^ masking_key[i % 4]);
    }

    Some((bytes_read, payload))
}
fn parse_ws_frame_full(frame: &[u8]) -> Option<(usize, WsFrameRecvd)> {
    let (fin, opcode, payload_length, payload_start_index, masking_key) = parse_ws_frame_header(frame)?;
    let (bytes_read, payload) = parse_ws_frame_body(frame, payload_length, payload_start_index, masking_key)?;
    Some((bytes_read, WsFrameRecvd {
        fin,
        opcode,
        payload
    }))
}

fn handle_websocket(mut bufread: BufReader<TcpStream>, mut buf: String, wsclients: &Arc<Mutex<Vec<MAHWebsocket>>>, patteval_update_tx: crossbeam_channel::Sender<PatternEvalUpdate>) {
    let sec_ws_key_header = "Sec-WebSocket-Key: ";
    let mut response = String::from("HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: ");
    while buf != "\r\n" { //line before data will have only \r\n (0D 0A)
        if buf.starts_with(sec_ws_key_header) {
            let ms = buf[sec_ws_key_header.len()..buf.len()-2].to_owned() + "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";
            response += {
                let mut s1hasher = Sha1::new();
                s1hasher.update(ms);
                let res = s1hasher.finalize();
                &base64::engine::general_purpose::STANDARD.encode(res)
            };
        }
        buf.clear();
        bufread.read_line(&mut buf).unwrap();
    }
    response+="\r\n\r\n";
    bufread.get_mut().write_all(response.as_bytes()).unwrap();
    bufread.get_mut().flush().unwrap();

    let uid = rand::random();
    println!("starting ws\t'{uid:#X}'");

    let wsrecvjh = {
        let wsclients = wsclients.clone();
        let mut tcpstream = bufread.get_mut().try_clone().unwrap();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(10)); //make sure this is in wsclients before recving close
            loop {
                let mut wsframebuf = vec![0; 65536]; //max frame size (not spec compliant)
                // std::thread::sleep(Duration::from_millis(1000)); // force frame coalescing in tcp stream
                let bytes_peeked = tcpstream.peek(&mut wsframebuf).unwrap();
                // println!("peeked {} bytes", bytes_peeked);
                if let Some((bytes_read, wsfr)) = parse_ws_frame_full(&wsframebuf[0..bytes_peeked]) {
                    tcpstream.read_exact(&mut wsframebuf[0..bytes_read]).unwrap(); //consume bytes
                    // assert!(wsfr.fin, "fragmented frames (fin=false) not yet implemented");
                    if !wsfr.fin {
                        eprintln!("fragmented frames (fin=false) not yet implemented, ignoring...");
                        continue;
                    }
                    match wsfr.opcode {
                        WsFrameOpcodes::Continuation => {
                            eprintln!("continuation frames not yet implemented, ignoring...");
                            continue;
                        }
                        WsFrameOpcodes::Text => patteval_update_tx.send(serde_json::from_slice(&wsfr.payload).unwrap()).unwrap(),
                        WsFrameOpcodes::Binary => todo!("binary ws frames"),
                        WsFrameOpcodes::Close => {
                            println!("closing ws\t'{uid:#X}'");
                            let mut wsclients = wsclients.lock().unwrap();
                            wsclients.retain(|pwso| pwso.uid != uid);
                            if wsclients.len() == 0 {
                                println!("no more ws clients, stopping playback");
                                patteval_update_tx.send(PatternEvalUpdate::Playstart { playstart: 0.0, playstart_offset: 0.0 }).unwrap();
                            }
                            break;
                        }
                        WsFrameOpcodes::Ping => {
                            let pong_frame = create_ws_frame(WsFrameOpcodes::Pong, &wsfr.payload);
                            tcpstream.write_all(&pong_frame).unwrap();
                        }
                        WsFrameOpcodes::Pong => {}
                    }
                }
            }
        })
    };

    let pws = MAHWebsocket { bufread, uid, _wsrecvjh: wsrecvjh };
    wsclients.lock().unwrap().push(pws);
}

fn loop_through_send_removing_fails(wsclients: &mut Vec<MAHWebsocket>, msg: &AdapticsWSServerMessage) {
    let len = wsclients.len();
    let mut del = 0;
    {
        for i in 0..len {
            if let Err(e) = wsclients[i].send(msg) {
                println!("removing wsclient: {i} for {e}");
                del += 1;
            } else if del > 0 {
                wsclients.swap(i - del, i);
            }
        }
    }
    if del > 0 {
        wsclients.truncate(len - del);
    }
}

fn websocket_dispatcher_loop_thread(
    wsclients: &Arc<Mutex<Vec<MAHWebsocket>>>,
    playback_updates_rx: &crossbeam_channel::Receiver<AdapticsWSServerMessage>,
    tracking_data_ws_rx: Option<&crossbeam_channel::Receiver<AdapticsWSServerMessage>>,
) {
    while let Ok(msg) =
        if let Some(tracking_data_ws_rx) = tracking_data_ws_rx.as_ref() {
            crossbeam_channel::select! {
                recv(playback_updates_rx) -> msg => msg,
                recv(tracking_data_ws_rx) -> msg => msg,
            }
        } else {
            playback_updates_rx.recv()
        }
    {
        loop_through_send_removing_fails(&mut wsclients.lock().unwrap(), &msg);
    }

    // channel disconnected so we should exit
}

pub fn start_ws_server(
    websocket_server_addr: &str,
    patteval_update_tx: &crossbeam_channel::Sender<PatternEvalUpdate>,
    playback_updates_rx: crossbeam_channel::Receiver<AdapticsWSServerMessage>,
    tracking_data_ws_rx: Option<crossbeam_channel::Receiver<AdapticsWSServerMessage>>,
) {
    let wsclients = Arc::new(Mutex::new(Vec::new()));
    {
        let wsclients = wsclients.clone();
        std::thread::spawn(move || websocket_dispatcher_loop_thread(&wsclients, &playback_updates_rx, tracking_data_ws_rx.as_ref()));
    }
    let listener = TcpListener::bind(websocket_server_addr).unwrap();
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let mut bufreader = BufReader::new(stream);
                let mut buf = String::new();
                bufreader.read_line(&mut buf).unwrap();
                if buf.starts_with("GET / HTTP/1.1") {
                    handle_websocket(bufreader, buf, &wsclients, patteval_update_tx.clone());
                } else {
                    println!("not websocket");
                }
            }
            Err(e) => {
                println!("error: {e}");
            }
        }
    }
    drop(listener);
}