#![allow(dead_code)]
#![allow(unused_imports)]

extern crate mio;
extern crate rustc_serialize;
extern crate byteorder;
pub mod dhtpackettypes;
pub mod sha;
pub mod packet;
use mio::*;
use mio::tcp::{TcpListener, TcpStream};
use std::collections::HashMap;
use byteorder::{BigEndian, WriteBytesExt, ByteOrder};
use packet::*;
use dhtpackettypes::*;
use sha::*;
use std::env;

const DHT_SERVER_SHAKE: u16 = 0x413f;
const DHT_CLIENT_SHAKE: u16 = 0x4121;
const LISTENER: Token = Token(0);
const CENTRAL_SERVER: Token = Token(1);

enum NodeState {
    AwaitingHandshake,
    Connected
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

struct MyHandler<'a> {
    central_server_socket: TcpStream,
    clients: HashMap<Token, TcpStream>,
    listener: TcpListener,
    token_counter: usize,
    state: NodeState,
    tcp_address: &'a str
}

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
                            handle_packet(&mut self.central_server_socket);
                        }
                    }
            },


            LISTENER => {
                //another node
                let mut neighbour_socket = match self.listener.accept() {
                        Err(e) => {
                            println!("Accept error: {}", e);
                            return;
                        },
                        Ok(None) => panic!("Accept has returned 'None'"),
                        Ok(Some(sock)) => sock
                    };

                //jos ei käytetä & niin me annetaan tämä socket tälle, joka laittaa sen hashmappiin
                //joka sitten omistaa socketin
                self.new(neighbour_socket, event_loop);

            },

            token => {
                match self.clients.get_mut(&token) {
                    Some(client_socket) => {handle_packet(client_socket);},
                    None => {println!("Failed to retrieve this socket from client list");}
                }
            }
        }
    }

}

impl<'a> MyHandler<'a> {
    fn new(&mut self, sock: TcpStream, event_loop: &mut EventLoop<MyHandler>) {
        self.token_counter += 1;
        let new_token = Token(self.token_counter);

        self.clients.insert(new_token, sock);
        event_loop.register_opt(&self.clients[&new_token], new_token,
                EventSet::readable(),PollOpt::edge() | PollOpt::oneshot()).unwrap();
    }
}

fn main() {
    //TODO: accept new connection from listener. left and right node. no need for hashmap?
    //
    let listening_port = env::args().nth(1).expect("Invalid number of arguments.");
    let server_addr = "127.0.0.1:9155".parse().unwrap();
    let tcp_address = "127.0.0.1".to_string() + ":" + &listening_port;
    let listener = TcpListener::bind(&tcp_address.parse().unwrap()).unwrap();
    let mut sock = TcpStream::connect(&server_addr).unwrap();

    let mut event_loop = EventLoop::new().unwrap();
    event_loop.register(&listener, LISTENER).unwrap();
    //edge triggered because we cant drain socket, it's just a buffer?
    event_loop.register_opt(&sock,
                        CENTRAL_SERVER,
                        EventSet::readable(),
                        PollOpt::edge()).unwrap();

    let mut handler = MyHandler {
        central_server_socket: sock,
        clients: HashMap::new(),
        listener: listener,
        token_counter: 1,
        state: NodeState::AwaitingHandshake,
        tcp_address: &tcp_address
    };

    event_loop.run(&mut handler).unwrap();
}
