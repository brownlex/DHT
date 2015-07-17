#![allow(dead_code)]
#![allow(unused_imports)]
extern crate mio;
extern crate sha1;
extern crate rustc_serialize;
extern crate byteorder;

use mio::*;
use mio::tcp::{TcpListener, TcpStream};
use std::collections::HashMap;
use std::cell::RefCell;
use std::rc::Rc;
use std::fmt;
use rustc_serialize::base64::{ToBase64, STANDARD};
use byteorder::{BigEndian, WriteBytesExt};
mod dhtpackettypes;
use dhtpackettypes::*;

const DHT_SERVER_SHAKE: u16 = 0x413f;
const DHT_CLIENT_SHAKE: u16 = 0x4121;
const LISTENER: Token = Token(0);
const CENTRAL_SERVER: Token = Token(1);

fn gen_key(ip: &String) -> [u8; 20] {
    let mut m = sha1::Sha1::new();
    let mut buf = [0u8; 20];

    m.update(ip.as_bytes());
    m.output(&mut buf);

    return buf;
    //return buf.to_base64(STANDARD);
}

enum NodeState {
    AwaitingHandshake,
    Connected
}

//'a needed so rust knows the elements inside have same lifetime as the struct?
struct DHTPacket<'a> {
    target_key: &'a [u8],
    sender_key: &'a [u8],
    request_type: u16,
    payload_length: u16,
    payload: &'a [u8]
}

impl<'a> DHTPacket<'a> {
    fn send_packet(&self, socket: &mut TcpStream) {
        let mut data: Vec<u8> = Vec::new();
        //map() maps the &i to i?
        data.extend(self.target_key.iter().map(|&i| i));
        data.extend(self.sender_key.iter().map(|&i| i));
        let mut type_as_bytes = vec![];
        let mut len_as_bytes = vec![];
        //vectors implement Write so this works?
        type_as_bytes.write_u16::<BigEndian>(self.request_type).unwrap();
        len_as_bytes.write_u16::<BigEndian>(self.payload_length).unwrap();
        data.extend(type_as_bytes);
        data.extend(len_as_bytes);
        data.extend(self.payload.iter().map(|&i| i));

        socket.try_write(&data[..]).unwrap();

    }
}

struct MyHandler<'a> {
    central_server_socket: TcpStream,
    clients: HashMap<Token, TcpStream>,
    token_counter: usize,
    state: NodeState,
    tcp_address: &'a str
}

fn register(handler: &mut MyHandler) {
    let sha_key = gen_key(&handler.tcp_address.to_string());
    let packet = DHTPacket {
        target_key: &sha_key[..],
        sender_key: &sha_key[..],
        request_type: DHT_REGISTER_BEGIN,
        payload_length: handler.tcp_address.len() as u16,
        payload: handler.tcp_address.as_bytes()
    };

    packet.send_packet(&mut handler.central_server_socket);
}

/*fn handle_packet() {
    
}*/

impl<'a> Handler for MyHandler<'a> {
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
                            self.central_server_socket.try_write("A!".as_bytes()).unwrap();
                            self.state = NodeState::Connected;
                            println!("handshake");
                            register(self);

                        },

                        NodeState::Connected => {
                            println!("got here");
                            self.state = NodeState::Connected;
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
    let server_addr = "127.0.0.1:9155".parse().unwrap();
    let listening_port = 9160;
    let tcp_address = "127.0.0.1".to_string() + ":" + &9160.to_string();
    let listener = TcpListener::bind(&tcp_address.parse().unwrap()).unwrap();
    let mut sock = TcpStream::connect(&server_addr).unwrap();

    let mut event_loop = EventLoop::new().unwrap();
    event_loop.register(&listener, LISTENER).unwrap();
    event_loop.register(&sock, CENTRAL_SERVER).unwrap();
    let mut handler = MyHandler {
        central_server_socket: sock,
        clients: HashMap::new(),
        token_counter: 0,
        state: NodeState::AwaitingHandshake,
        tcp_address: &tcp_address
    };

    event_loop.run(&mut handler).unwrap();
}
