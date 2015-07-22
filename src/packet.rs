use mio::*;
use mio::tcp::{TcpListener, TcpStream};
use std::collections::HashMap;
use byteorder::{BigEndian, WriteBytesExt, ByteOrder};
use sha::*;
use dhtpackettypes::*;
use std::str::from_utf8;

//'a needed so rust knows the elements inside have same lifetime as the struct?
pub struct DHTPacket<'a> {
    pub target_key: &'a [u8],
    pub sender_key: &'a [u8],
    pub request_type: u16,
    pub payload_length: u16,
    pub payload: &'a [u8]
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

    socket.try_write(&data[..]).unwrap();
}

impl<'a> DHTPacket<'a> {
    pub fn send_packet(&self, socket: &mut TcpStream) {
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

pub fn handle_packet(socket: &mut TcpStream) {
    let mut data: Vec<u8> = vec![];
    loop {
        let mut buf = [0; 2048];
        match socket.try_read(&mut buf) {
            Err(e) => {
                println!("Error while reading socket: {:?}", e);
                return
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

    let request_type = BigEndian::read_u16(&data[40..42]);
    let payload_length = BigEndian::read_u16(&data[42..44]) as usize;

    match request_type {
        DHT_REGISTER_FAKE_ACK => {
            send_packet(socket, &data[0..20], &data[20..40], DHT_REGISTER_DONE, 0, &[]);
        },

        DHT_REGISTER_BEGIN => {
            println!("New node joined");
            //tässä jossain panikoi
            let payload = from_utf8(&data[44..44 + payload_length]).unwrap();
            println!("{}", payload);
            let client_addr = payload.parse().unwrap();
            let mut new_client = TcpStream::connect(&client_addr).unwrap();
            send_packet(&mut new_client, &data[0..20], &data[20..40], DHT_REGISTER_ACK, 0, &[]);

        },
        _ => {
            println!("request_type not expected");
        }
    }

}