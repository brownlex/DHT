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
use std::io;
use std::thread;

fn print_instructions() {
    println!("Input a number:");
    println!("1. Deregister");
}

fn main() {
    let listening_port = env::args().nth(1).expect("Invalid number of arguments.");
    let server_addr = "127.0.0.1:9155".parse().unwrap();
    let tcp_address = "127.0.0.1".to_string() + ":" + &listening_port;
    let listener = TcpListener::bind(&tcp_address.parse().unwrap()).unwrap();
    let sock = TcpStream::connect(&server_addr).unwrap();

    let mut event_loop = EventLoop::new().unwrap();
    let sender = event_loop.channel();

    event_loop.register_opt(&listener,
                        LISTENER,
                        EventSet::readable(),
                        PollOpt::edge()).unwrap();
    //edge triggered because we cant drain socket, it's just a buffer?
    event_loop.register_opt(&sock,
                        CENTRAL_SERVER,
                        EventSet::readable(),
                        PollOpt::edge()).unwrap();

    let server = Client {
        socket: sock,
        state: ClientState::Connected,
        interest: EventSet::readable(),
        sending_data: vec![]
    };

    let mut clients = HashMap::new();
    clients.insert(CENTRAL_SERVER, server);

    let node = Node {
        listener: listener,
        state: NodeState::AwaitingHandshake,
        node_key: gen_key(&tcp_address),
        tcp_address: tcp_address,
        token_counter: 1,
        clients: clients
    };

    thread::spawn(move || {
        loop {
            let mut input = String::new();
            let _ = io::stdin().read_line(&mut input).ok().expect("Failed to read line");
            let input_number: u32 = input.parse::<u32>().unwrap();
            let _ = sender.send(input_number);
        }
    });

    let mut handler = MyHandler {
        node: node
    };

    print_instructions();
    event_loop.run(&mut handler).unwrap();
}
