use std::{io::{Read, Write}, net::TcpStream, sync::{Arc, Mutex}};
use crate::structs::{ConvertVecToMessageStruct, Message, MessageKind};

pub fn handle_connection(my_addr:String,mut stream: TcpStream, msg_list:Arc<Mutex<Vec<Message>>>) {
    let mut read_buf = [0u8; 2048];
    match stream.read(&mut read_buf) {
        Ok(n) => {
            if n > 0 { 
                let income_data=read_buf[0..n].to_vec().to_struct();
                msg_list.lock().unwrap().push(income_data);
                stream.write(&Message{
                    id: 0,
                    sender: my_addr.clone(),
                    kind: MessageKind::Ok,
                    payload: Vec::new(),
                }.to_byte_array()).unwrap();
            }
        }
        Err(_err) => {
            stream.write_all("ERR".as_bytes()).unwrap();
        }
    }
}

