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
use std::net::SocketAddr;

pub const DHT_SERVER_SHAKE: u16 = 0x413f;
pub const DHT_CLIENT_SHAKE: u16 = 0x4121;
pub const LISTENER: Token = Token(0);
pub const CENTRAL_SERVER: Token = Token(1);

pub enum NodeState {
    AwaitingHandshake,
    Registering,
    OneAck,
    Connected
}

pub enum ClientState {
    Registering,
    OneAck,
    Connected
}

pub struct Client {
    pub socket: TcpStream,
    pub interest: EventSet,
    pub state: ClientState,
    /* this is used when we connect to a new socket. we cant immediately write to the new socket
    because sometimes it cant connect before writing to it. we use this part instead and save the data
    we want to send to the new socket here, and when it triggers a writable event we write this data to the socket */
    pub client_key: [u8; 20],
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
                    //println!("{}", len);
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
        _ => {}
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

fn connect_and_prepare_packet(payload: &str, node: &mut Node, sending_data: Vec<u8>,
                                event_loop: &mut EventLoop<MyHandler>) {
    let client_addr: SocketAddr = payload.parse().unwrap();
    match TcpStream::connect(&client_addr) {
        Ok(sock) => {
            let mut new_client_sock = sock;
            //writable at first because writing to a socket instantly after connecting sometimes fails
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
}

pub fn handle_packet (token: Token, node: &mut Node, event_loop: &mut EventLoop<MyHandler>) {
    let data = node.clients.get_mut(&token).unwrap().read();
    if data.len() < 44 {return} //if whole packet didnt get here we ignore it.. patchwork

    let request_type = BigEndian::read_u16(&data[40..42]);
    let payload_length = BigEndian::read_u16(&data[42..44]) as usize;

    match request_type {
        //we are the first node
        DHT_REGISTER_FAKE_ACK => {
            let mut server = node.clients.get_mut(&CENTRAL_SERVER).unwrap();
            node.state = NodeState::Connected;
            send_packet(&mut server.socket, &data[0..20], &data[20..40], DHT_REGISTER_DONE, 0, &[]);
        },

        /* new node, we connect to it and mark it as writable so when
            it's ready we can send the ACK-packet */
        DHT_REGISTER_BEGIN => {
            let payload = from_utf8(&data[44..44 + payload_length]).unwrap();
            println!("Node's ip: {}", payload);
            let sending_data = make_packet(&data[0..20], &data[20..40], DHT_REGISTER_ACK, 0, &[]);
            connect_and_prepare_packet(payload, node, sending_data, event_loop);

        },

        //neighbour nodes acknowledge register
        DHT_REGISTER_ACK => {
            match node.state {
                NodeState::Registering => {
                    let mut server = node.clients.get_mut(&CENTRAL_SERVER).unwrap();
                    send_packet(&mut server.socket, &data[0..20], &data[20..40], DHT_REGISTER_DONE, 0, &[]);
                    node.state = NodeState::Connected;
                },

                _ => {
                    println!("Already sent register done, ignoring");
                }
            }
        },

        //server responded and we begin deregistering
        DHT_DEREGISTER_ACK => {
            let midpoint = 44 + payload_length/2;
            let first_address = from_utf8(&data[44..midpoint]).unwrap();
            let second_address = from_utf8(&data[midpoint..44+payload_length]).unwrap();
            //shiiet connect ja mark as writable ja sendii samalla tavalla?
            let first_sending_data = make_packet(&node.node_key, &node.node_key, DHT_DEREGISTER_BEGIN, 0, &[]);
            connect_and_prepare_packet(first_address, node, first_sending_data, event_loop);

            let second_sending_data = make_packet(&node.node_key, &node.node_key, DHT_DEREGISTER_BEGIN, 0, &[]);
            connect_and_prepare_packet(first_address, node, second_sending_data, event_loop);
        },

        //a node wants to deregister
        DHT_DEREGISTER_BEGIN => {
            let client_key = node.clients.get_mut(&token).unwrap().
            let mut server = node.clients.get_mut(&CENTRAL_SERVER).unwrap();
            send_packet(&mut server.socket, &data[0..20], &node.node_key, DHT_DEREGISTER_DONE, 0, &[]);
        }

        _ => {
            println!("request type not expected");
        }
    }

}

fn register(node: &mut Node) {
    let sha_key = gen_key(&node.tcp_address);
    let mut client = node.clients.get_mut(&CENTRAL_SERVER).unwrap();

    send_packet(&mut client.socket, &sha_key[..], &sha_key[..], DHT_REGISTER_BEGIN,
                node.tcp_address.len() as u16,
                node.tcp_address.as_bytes());
}

fn deregister(node: &mut Node) {
    let mut server = node.clients.get_mut(&CENTRAL_SERVER).unwrap();
    send_packet(&mut server.socket, &node.node_key, &node.node_key, DHT_DEREGISTER_BEGIN, 0, &[])
}

pub struct MyHandler {
    pub node: Node 
}

impl Handler for MyHandler   {
    type Timeout = ();
    type Message = u32;

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
                                self.node.state = NodeState::Registering;
                                }
                                register(&mut self.node);

                            },

                            _ => {
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
                            //remove from hashtable? as we send only one packet?
                        },

                        _ => {
                            println!("lel");
                        }
                    }
                }
            }
        }
    }

    fn notify(&mut self, event_loop: &mut EventLoop<MyHandler>, msg: u32) {
        match msg {
            1 => {
                println!("Starting deregister sequence");
                deregister(&mut self.node);
            },

            _ => {
                println!("Command not found");
            }
        }
        //event_loop.shutdown();
    }

}