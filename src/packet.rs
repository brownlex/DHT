extern crate mio;

use mio::*;
use mio::tcp::{TcpListener, TcpStream};
use std::collections::HashMap;
use byteorder::{BigEndian, WriteBytesExt, ByteOrder};
use sha::*;
use dhtpackettypes::*;
use std::str::from_utf8;
use std::io;
use std::thread;

pub const DHT_SERVER_SHAKE: u16 = 0x413f;
pub const DHT_CLIENT_SHAKE: u16 = 0x4121;
pub const LISTENER: Token = Token(0);
pub const CENTRAL_SERVER: Token = Token(1);

pub enum NodeState {
    AwaitingHandshake,
    Connected
}

pub enum ClientState {
    Registering,
    Connected
}

pub struct Client {
    pub socket: TcpStream,
    pub interest: EventSet,
    pub state: ClientState,
    /* this is used when we connect to a new socket. we cant immediately write to the new socket
    because sometimes it cant connect before writing to it. we use this part instead and save the data
    we want to send to the new socket here, and when it triggers a writable event we write this data to the socket */
    pub sending_data: Vec<u8> 

}

impl Client {
    fn read(&mut self) -> Vec<u8> {
        let mut data: Vec<u8> = vec![];
        loop {
            let mut buf = [0; 2048];
            match self.socket.try_read(&mut buf) {
                Err(e) => {
                    println!("Error while reading socket: {:?}", e);
                    break;
                },
                Ok(None) => 
                    // Socket buffer has got no more bytes.
                    break,
                Ok(Some(len)) => {
                    let slice = &buf[0..len];
                    //how to make sure only one packet is received at a time?
                    //move reading to main handler and use payload length to divide packets?
                    //but sometimes whole packet doesn't come at once?
                    println!("{}", len);
                    data.extend(slice.iter().map(|&i| i));
                }
            }
        }

        data
    }

    fn write(&mut self) {
        match self.socket.try_write(&self.sending_data) {
            Err(e) => {println!("Error while writing to a socket: {:?}", e);},
            _ => {println!("Write ok");}
        }
    }
}
//TODO: write/read for client instead of in program?

pub struct Node {
    pub token_counter: usize,
    pub clients: HashMap<Token, Client>,
    pub listener: TcpListener,
    pub state: NodeState,
    pub tcp_address: String,
    pub node_key: [u8; 20]
}

impl Node  {
    pub fn new_client(&mut self, client: Client, event_loop: &mut EventLoop<MyHandler>) {
        self.token_counter += 1;
        let new_token = Token(self.token_counter);

        self.clients.insert(new_token, client);
        event_loop.register_opt(&self.clients[&new_token].socket, new_token,
                    self.clients[&new_token].interest, PollOpt::edge() | PollOpt::oneshot()).unwrap();
    }

}

pub fn send_packet(socket: &mut TcpStream,
                target_key: &[u8], 
                sender_key: &[u8],
                request_type: u16,
                payload_length: u16,
                payload: &[u8])
{
    let mut data: Vec<u8> = Vec::new();
    //map() maps the &i to i?
    data.extend(target_key.iter().map(|&i| i));
    data.extend(sender_key.iter().map(|&i| i));
    let mut type_as_bytes = vec![];
    let mut len_as_bytes = vec![];
    //vectors implement Write so this works?
    type_as_bytes.write_u16::<BigEndian>(request_type).unwrap();
    len_as_bytes.write_u16::<BigEndian>(payload_length).unwrap();
    data.extend(type_as_bytes);
    data.extend(len_as_bytes);
    data.extend(payload.iter().map(|&i| i));

    match socket.try_write(&data[..]) {
        Err(e) => {println!("Error while writing to a socket: {:?}", e);},
        _ => {println!("Write ok");}
    }
}

pub fn make_packet(target_key: &[u8], 
                sender_key: &[u8],
                request_type: u16,
                payload_length: u16,
                payload: &[u8]) -> Vec<u8>
{
    let mut data: Vec<u8> = Vec::new();
    //map() maps the &i to i?
    data.extend(target_key.iter().map(|&i| i));
    data.extend(sender_key.iter().map(|&i| i));
    let mut type_as_bytes = vec![];
    let mut len_as_bytes = vec![];
    //vectors implement Write so this works?
    type_as_bytes.write_u16::<BigEndian>(request_type).unwrap();
    len_as_bytes.write_u16::<BigEndian>(payload_length).unwrap();
    data.extend(type_as_bytes);
    data.extend(len_as_bytes);
    data.extend(payload.iter().map(|&i| i));

    data
}

pub fn handle_packet (token: Token, node: &mut Node, event_loop: &mut EventLoop<MyHandler>) {
    let data = node.clients.get_mut(&token).unwrap().read();
    if data.len() == 0 {return}

    let request_type = BigEndian::read_u16(&data[40..42]);
    let payload_length = BigEndian::read_u16(&data[42..44]) as usize;

    match request_type {
        DHT_REGISTER_FAKE_ACK => {
            let mut server = node.clients.get_mut(&CENTRAL_SERVER).unwrap();
            send_packet(&mut server.socket, &data[0..20], &data[20..40], DHT_REGISTER_DONE, 0, &[]);
        },

        DHT_REGISTER_BEGIN => {
            println!("New node joined");
            let payload = from_utf8(&data[44..44 + payload_length]).unwrap();
            println!("Node's ip: {}", payload);
            let client_addr = payload.parse().unwrap();
            /* new node, we connect to it and mark it as writable so when
            it's ready we can send the ACK-packet */
            match TcpStream::connect(&client_addr) {
                Ok(sock) => {
                    let mut new_client_sock = sock;
                    //writable at first because writing to a socket instantly after connecting sometimes fails
                    let sending_data = make_packet(&data[0..20], &data[20..40], DHT_REGISTER_ACK, 0, &[]);
                    let client = Client {
                        socket: new_client_sock,
                        interest: EventSet::writable(),
                        state: ClientState::Registering,
                        sending_data: sending_data
                    };

                    node.new_client(client, event_loop);
                },
                Err(e) => {
                    println!("Error trying to connect neighbour: {:?}", e);
                }
            }

        },

        DHT_REGISTER_ACK => {
            //we need two acks, this is just temporary
            let mut server = node.clients.get_mut(&CENTRAL_SERVER).unwrap();
            send_packet(&mut server.socket, &data[0..20], &data[20..40], DHT_REGISTER_DONE, 0, &[])
        },

        _ => {
            println!("request type not expected");
        }
    }

}

fn register(node: &mut Node) {
    let sha_key = gen_key(&node.tcp_address);
    let mut client = node.clients.get_mut(&CENTRAL_SERVER).unwrap();

    send_packet(&mut client.socket,
                &sha_key[..],
                &sha_key[..],
                DHT_REGISTER_BEGIN,
                node.tcp_address.len() as u16,
                node.tcp_address.as_bytes());
}

pub struct MyHandler {
    pub node: Node 
}

impl Handler for MyHandler   {
    type Timeout = ();
    type Message = String;

    fn ready(&mut self, event_loop: &mut EventLoop<MyHandler>, token: Token, events: EventSet) {
        if events.is_readable() {
            match token {
                CENTRAL_SERVER => {
                    // server answered
                    match self.node.state {
                            NodeState::AwaitingHandshake => {
                                let mut buf = [0; 2048];
                                //the handshake is always "A?", we just send "A!" back
                                //scoping so borrowing only here and register can take it
                                {
                                let mut client = self.node.clients.get_mut(&CENTRAL_SERVER).unwrap();
                                client.socket.try_read(&mut buf).unwrap();
                                client.socket.try_write("A!".as_bytes()).unwrap();
                                self.node.state = NodeState::Connected;
                                println!("handshake");
                                }
                                register(&mut self.node);

                            },

                            NodeState::Connected => {
                                handle_packet(CENTRAL_SERVER, &mut self.node, event_loop);
                            }
                        }
                },


                LISTENER => {
                    //another node connected
                    let mut neighbour_socket = match self.node.listener.accept() {
                            Err(e) => {
                                println!("Accept error: {}", e);
                                return;
                            },
                            Ok(None) => panic!("Accept has returned 'None'"),
                            Ok(Some(sock)) => sock
                        };
                    println!("Accepted something");
                    //tässä pitää olla readable aluksi koska odotat viestiä toiselta
                    let client = Client {
                        socket: neighbour_socket,
                        interest: EventSet::readable(),
                        state: ClientState::Registering,
                        sending_data: vec![]
                    };

                    self.node.new_client(client, event_loop);

                },

                token => {
                    handle_packet(token, &mut self.node, event_loop);
                    event_loop.reregister(&self.node.clients[&token].socket, token, EventSet::readable(),
                                  PollOpt::edge() | PollOpt::oneshot()).unwrap();
                }
            }
        }

        if events.is_writable() {
            match token {
                token => {
                    let mut client = self.node.clients.get_mut(&token).unwrap();
                    match client.state {
                        ClientState::Registering => {
                            client.write();
                            client.state = ClientState::Connected;
                        },

                        ClientState::Connected => {
                            println!("lel");
                        }
                    }
                }
            }
        }
    }

    fn notify(&mut self, event_loop: &mut EventLoop<MyHandler>, msg: String) {
        println!("{}", msg);
        event_loop.shutdown();
    }

}