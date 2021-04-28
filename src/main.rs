extern crate rosc;
extern crate reqwest;
extern crate url;

use rosc::{OscPacket, OscMessage};
use reqwest::blocking::ClientBuilder;
use retry::retry;

use std::env;
use std::net::{SocketAddrV4, UdpSocket};
use std::str::FromStr;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;


const API_TIMEOUT_SECONDS: u64 = 1;
const API_RETRY_MS: u64 = 100;
const API_RETRY_COUNT: usize = 3;

enum VmixMessage {
    Fader(i32),
    CutToInput(String),
    PreviewInput(String),
    Raw(String),
}

fn vmix_api_client(server: String, rx: mpsc::Receiver<VmixMessage>) {
    let timeout = Duration::new(API_TIMEOUT_SECONDS, 0);
    let client = ClientBuilder::new().timeout(timeout).build().unwrap();

    let server_url_prefix = format!("http://{server}/api",
        server = server
    );

    loop {
        let api_request = match rx.recv().unwrap() {
            VmixMessage::Fader(x) => format!("Function=SetFader&Value={}", x),
            VmixMessage::CutToInput(x) => format!("Function=CutDirect&Input={}", x),
            VmixMessage::PreviewInput(x) => format!("Function=PreviewInput&Input={}", x),
            VmixMessage::Raw(x) => format!("{}", x),
        };
        let server_url = format!("{url_prefix}?{api_request}", url_prefix = server_url_prefix, api_request = api_request);

        println!("TX: INFO: request = {:?}", server_url);
        let resp = retry(retry::delay::Fixed::from_millis(API_RETRY_MS).take(API_RETRY_COUNT), || client.get(&server_url).send());
        match resp {
          Ok(_) => {},
          Err(e) => println!("TX: ERR: Error while invoking API request \"{request}\": {err}", request=server_url, err=e)
        }
    }
}

fn main() {
    //
    // OSC initialization
    // 
    let args: Vec<String> = env::args().collect();
    let usage = format!("Usage {} LISTEN-IP:PORT VMIX-IP:PORT", &args[0]);
    if args.len() < 3 {
        println!("{}", usage);
        ::std::process::exit(1)
    }
    let listen_addr = match SocketAddrV4::from_str(&args[1]) {
        Ok(listen_addr) => listen_addr,
        Err(_) => panic!(usage),
    };
    let sock = UdpSocket::bind(listen_addr).unwrap();
    println!("Listening to {}", listen_addr);

    let mut buf = [0u8; rosc::decoder::MTU];

    //
    // Create channel and spawn vMix API client thread
    //
    let (tx, rx) = mpsc::channel();
    let server_url = args[2].clone();

    thread::spawn(|| {
        vmix_api_client(server_url, rx)
    });

    //
    // Main loop -- receive and handle OSC packets
    //
    loop {
        match sock.recv_from(&mut buf) {
            Ok((size, _addr)) => {
                let packet = rosc::decoder::decode(&buf[..size]).unwrap();
                handle_packet(packet, &tx);
            }
            Err(e) => {
                println!("RX: ERR: Error receiving from socket: {}", e);
                break;
            }
        }
    }
}

//
// Handle OSC packet. Do error handling and then pass to vMix.
//
fn handle_packet(packet: OscPacket, tx: &mpsc::Sender<VmixMessage>) {
    match packet {
        OscPacket::Message(msg) => {
            println!("RX: INFO: Received addr {} args {:?}", msg.addr, msg.args);

            match msg.addr.as_str() {
                "/vmix/fader" => handle_fader_message(msg, tx),
                "/vmix/cut" => handle_cut_message(msg, tx),
                "/vmix/preview" => handle_preview_message(msg, tx),
                "/vmix/raw" => handle_raw_message(msg, tx),
                _ => println!("RX: ERR: Received unknown OSC address {}, ignoring", msg.addr),
            }
        }
        OscPacket::Bundle(bundle) => {
            println!("RX: ERR: Rexeived OSC bundle. OSC bundles currently not supported.  Bundle: {:?}", bundle);
        }
    }
}

//
// Breakout functions to handle specific requests and validate arguments
//
fn handle_fader_message(msg: OscMessage, tx: &mpsc::Sender<VmixMessage>) {
  if msg.args.len() == 1 {
    match msg.args[0] {
      rosc::OscType::Int(val) => tx.send(VmixMessage::Fader(val)).unwrap(),
      rosc::OscType::Float(val) => tx.send(VmixMessage::Fader(val.trunc() as i32)).unwrap(),
      _ => println!("RX: ERR: Received OSC message \"/vmix/fader\" with unsupported value type. Received {:?}, expected integer or float", msg.args[0]),
    }
  } else {
    println!("RX: ERR: Received OSC message \"/vmix/fader\" with invalid number of arguments. Expected one argument, got {}", msg.args.len());
  }
}

fn handle_cut_message(msg: OscMessage, tx: &mpsc::Sender<VmixMessage>) {
  if msg.args.len() == 1 {
    match &msg.args[0] {
      rosc::OscType::Int(val) => tx.send(VmixMessage::CutToInput(val.to_string())).unwrap(),
      rosc::OscType::String(val) => tx.send(VmixMessage::CutToInput(val.clone())).unwrap(),
      _ => println!("RX: ERR: Received OSC message \"/vmix/cut\" with unsupported value type. Received {:?}, expected integer or string", msg.args[0]),
    }
  } else {
    println!("RX: ERR: Received OSC message \"/vmix/cut\" with invalid number of arguments. Expected one argument, got {}", msg.args.len());
  }
}

fn handle_preview_message(msg: OscMessage, tx: &mpsc::Sender<VmixMessage>) {
  if msg.args.len() == 1 {
    match &msg.args[0] {
      rosc::OscType::Int(val) => tx.send(VmixMessage::PreviewInput(val.to_string())).unwrap(),
      rosc::OscType::String(val) => tx.send(VmixMessage::PreviewInput(val.clone())).unwrap(),
      _ => println!("RX: ERR: Received OSC message \"/vmix/preview\" with unsupported value type. Received {:?}, expected integer or string", msg.args[0]),
    }
  } else {
    println!("RX: ERR: Received OSC message \"/vmix/preview\" with invalid number of arguments. Expected one argument, got {}", msg.args.len());
  }
}

fn handle_raw_message(msg: OscMessage, tx: &mpsc::Sender<VmixMessage>) {
  if msg.args.len() == 1 {
    match &msg.args[0] {
      rosc::OscType::String(val) => tx.send(VmixMessage::Raw(val.clone())).unwrap(),
      _ => println!("RX: ERR: Received OSC message \"/vmix/raw\" with unsupported value type. Received {:?}, expected string", msg.args[0]),
    }
  } else {
    println!("RX: ERR: Received OSC message \"/vmix/raw\" with invalid number of arguments. Expected one argument, got {}", msg.args.len());
  }
}
