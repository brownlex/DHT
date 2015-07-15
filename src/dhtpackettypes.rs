pub const DHT_REGISTER_BEGIN: u16 = 1;
pub const DHT_REGISTER_ACK: u16 = 2;
pub const DHT_REGISTER_FAKE_ACK: u16 = 3;
pub const DHT_REGISTER_DONE: u16 = 4;
 
pub const DHT_DEREGISTER_BEGIN: u16 = 11;
pub const DHT_DEREGISTER_ACK: u16 = 12;
pub const DHT_DEREGISTER_DONE: u16 = 13;
pub const DHT_DEREGISTER_DENY: u16 = 14;	
 
pub const DHT_GET_DATA: u16 = 21;
pub const DHT_PUT_DATA: u16 = 22;
pub const DHT_DUMP_DATA: u16 = 23;
pub const DHT_PUT_DATA_ACK: u16 = 24;
pub const DHT_DUMP_DATA_ACK: u16 = 25;
pub const DHT_SEND_DATA: u16 = 26;
pub const DHT_TRANSFER_DATA: u16 = 27;
pub const DHT_NO_DATA: u16 = 28;
 	
pub const DHT_ACQUIRE_REQUEST: u16 = 31;
pub const DHT_ACQUIRE_ACK: u16 = 32;
pub const DHT_RELEASE_REQUEST: u16 = 33;
pub const DHT_RELEASE_ACK: u16 = 34;
 
pub const DHT_LIST_RESOURCES: u16 = 35;