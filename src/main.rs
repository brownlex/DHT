#![allow(dead_code)]
#![allow(unused_imports)]
extern crate mio;
extern crate sha1;
extern crate rustc_serialize;
extern crate byteorder;

use mio::*;
use mio::tcp::*;
use mio::tcp::TcpStream::*;
use std::collections::HashMap;
use std::cell::RefCell;
use std::rc::Rc;
use std::fmt;
use rustc_serialize::base64::{ToBase64, STANDARD};
use byteorder::ByteOrder;
mod dhtpackettypes;
use std::str;

const DHT_SERVER_SHAKE: u16 = 0x413f;
const DHT_CLIENT_SHAKE: u16 = 0x4121;
const CENTRAL_SERVER: Token = Token(0);

fn gen_key(ip: &String) -> String {
    let mut m = sha1::Sha1::new();
    let mut buf = [0u8; 20];

    //m.update(key.as_bytes());
    m.update("258EAFA5-E914-47DA-95CA-C5AB0DC85B11".as_bytes());

    m.output(&mut buf);

    return buf.to_base64(STANDARD);
}

enum NodeState {
    AwaitingHandshake,
    Registering,
    Connected
}

struct MyHandler {
    central_server_socket: TcpStream,
    clients: HashMap<Token, TcpStream>,
    token_counter: usize,
    state: NodeState
}

impl Handler for MyHandler {
	type Timeout = ();
    type Message = ();

    fn ready(&mut self, event_loop: &mut EventLoop<MyHandler>, token: Token, _: EventSet) {
        match token {
            CENTRAL_SERVER => {
                // server answered
                match self.state {
                        NodeState::AwaitingHandshake => {
                            let mut buf = [0; 2048];
                            //the handshake is always "A?", we just send "A!" back
                            self.central_server_socket.try_read(&mut buf).unwrap();
                            self.central_server_socket.try_write("A!".as_bytes());
                            self.state = NodeState::Registering;
                            println!("handshake");
                        },

                        NodeState::Registering => {
                            self.state = NodeState::Registering;
                        },

                        NodeState::Connected => {
                            self.state = NodeState::Registering;
                        }
                    }
            },
            _ => panic!("unexpected token"),
        }
    }
}

fn main() {
    //connect to the server, create eventloop and register this connection
    //to the loop and wait for the server answer
    let addr = "127.0.0.1:9155".parse().unwrap();
    let mut event_loop = EventLoop::new().unwrap();
    let mut sock = TcpStream::connect(&addr).unwrap();

    event_loop.register(&sock, CENTRAL_SERVER).unwrap();
    let mut handler = MyHandler {
        central_server_socket: sock,
        clients: HashMap::new(),
        token_counter: 0,
        state: NodeState::AwaitingHandshake
    };

    event_loop.run(&mut handler).unwrap();
}
