extern crate rosc;
extern crate reqwest;

use rosc::OscPacket;
use reqwest::blocking::Client;

use std::env;
use std::net::{SocketAddrV4, UdpSocket};
use std::str::FromStr;
use std::sync::mpsc;
use std::thread;

enum VmixMessage {
    Fader(i32),
    CutToInput(String),
    PreviewInput(String),
    Raw(String),
}

fn vmix_api_client(server: String, rx: mpsc::Receiver<VmixMessage>) {
    let client = Client::new();

    // http://10.4.132.189:8088/api/?Function=PreviewInput&Input=1
    // http://10.4.132.189:8088/api/?Function=SetFader&Value=1
    // http://10.4.132.189:8088/api/?Function=Fade&Duration=6000
    let server_url_prefix = format!("http://{server}/api",
        server = server
    );

    loop {
        let api_request = match rx.recv() {
            Ok(api_request) => api_request,
            Err(error) => panic!("Error receiving message: {:?}", error),
        };
        let api_request = match rx.recv().unwrap() {
            VmixMessage::Fader(x) => format!("Function=SetFader&Value={x}", x=x),
            VmixMessage::CutToInput(x) => format!("Function=XXXXPreviewInput&Input={x}", x=x),
            VmixMessage::PreviewInput(x) => format!("Function=PreviewInput&Input={x}", x=x),
            VmixMessage::Raw(x) => format!("{x}", x=x),
        };
        let server_url = format!("{url_prefix}?{api_request}", url_prefix = server_url_prefix, api_request = api_request);

        println!("request = {:?}", server_url);
        let body = client.get(&server_url).send();
        println!("body = {:?}", body);
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
    // Channel
    //
    let (tx, rx) = mpsc::channel();
    let server_url = args[2].clone();

    thread::spawn(|| {
        vmix_api_client(server_url, rx)
    });

    loop {
        match sock.recv_from(&mut buf) {
            Ok((size, addr)) => {
                println!("Received packet with size {} from: {}", size, addr);
                let packet = rosc::decoder::decode(&buf[..size]).unwrap();
                handle_packet(packet, &tx);
            }
            Err(e) => {
                println!("Error receiving from socket: {}", e);
                break;
            }
        }
    }
}

fn handle_packet(packet: OscPacket, tx: &mpsc::Sender<VmixMessage>) {
    match packet {
        OscPacket::Message(msg) => {
            println!("OSC address: {}", msg.addr);
            println!("OSC arguments: {:?}", msg.args);

            match msg.addr.as_str() {
                "/fader" => tx.send(VmixMessage::Fader(msg.args[0].clone().int().unwrap())).unwrap(),
                "/cut" => tx.send(VmixMessage::CutToInput(msg.args[0].clone().string().unwrap())).unwrap(),
                "/preview" => tx.send(VmixMessage::PreviewInput(msg.args[0].clone().string().unwrap())).unwrap(),
                "/raw" => tx.send(VmixMessage::Raw(msg.args[0].clone().string().unwrap())).unwrap(),
                _ => println!("Received unknown OSC address {}, ignoring", msg.addr),
            }
        }
        OscPacket::Bundle(bundle) => {
            println!("OSC Bundle: {:?}", bundle);
        }
    }
}
