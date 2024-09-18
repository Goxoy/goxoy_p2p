use std::{io::{Read, Write}, net::TcpStream, time::{Duration, SystemTime}};
use crate::structs::{ConvertVecToMessageStruct, Message, MessageKind, NodeDetails, NodeStatus};

pub fn control_nodes_hash(node_list:Vec<NodeDetails>)->bool{
    let mut all_equal=true;
    let mut current_hash=String::new();
    for n_info in node_list.iter(){
        if n_info.status==NodeStatus::Online{
            if current_hash.len()==0{
                current_hash=n_info.node_hash.clone();
            }
            if current_hash.eq(&n_info.node_hash)==false{
                all_equal=false;
            }
        }
    } 
    all_equal
}


pub fn client(node_addr:String,msg_data: Vec<u8>,) -> Message {
    #[allow(unused_assignments)]
    let mut result_no=0;
    match TcpStream::connect(node_addr.clone()) {
        Ok(mut stream) => {
            match stream.set_read_timeout(Some(Duration::from_millis(10))) {
                Ok(_) => {},
                Err(_) => {},
            }

            match stream.write(&msg_data){
                Ok(_) => {
                    let mut data = [0u8; 2048];
                    match stream.read(&mut data) {
                        Ok(data_size) => {
                            if data_size==0{
                                result_no=77;
                                _ = stream.flush();
                                _ = stream.shutdown(std::net::Shutdown::Both);        
                            }else{
                                let convert_buf=data[0..data_size].to_vec();
                                let c_data=convert_buf.to_struct();
                                return c_data;
                            }
                        },
                        Err(_e) => {
                            match stream.flush(){
                                Ok(_a) => {},
                                Err(_e) => { /* println!("from stream.flush() => {}",e) */ },
                            }
                            match stream.shutdown(std::net::Shutdown::Read){
                                Ok(_n) => {},
                                Err(_e) => { /* println!("from stream.shutdown => {}",e) */ },
                            }
                            result_no=9;
                        }
                    }        
                },
                Err(_) => {
                    result_no=8;
                },
            }
        },
        Err(_e) => {
            result_no=5;
        }
    }
    Message{
        id: result_no,
        sender: String::new(),
        kind: MessageKind::Error,
        payload: Vec::new(),
    }        
}

pub fn calculate_node_list_hash(node_list:Vec<NodeDetails>)->String{
    let mut tmp_status=Vec::new();
    let mut tmp_list=Vec::new();
    for n_info in node_list.iter(){
        tmp_list.push(n_info.addr.clone());
        tmp_status.push(
            format!("{}:{}",
                n_info.addr.clone(),
                n_info.status.to_string()
            )
        );
    }
    tmp_list.sort();
    tmp_status.sort();
    let list_hash=format!("{:x}", md5::compute(serde_json::to_string(&tmp_list).unwrap()));
    let status_hash=format!("{:x}", md5::compute(serde_json::to_string(&tmp_status).unwrap()));
    format!("{}:{}",&list_hash[..8],&status_hash[..8],)
}

pub fn get_sys_time_in_secs() -> u128 {
    match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
        Ok(n) => n.as_secs() as u128,
        Err(_) => 0,
    }
}

pub fn get_sys_time_in_millis() -> u128 {
    match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
        Ok(n) => n.as_millis(),
        Err(_) => 0,
    }
}

pub fn get_sys_time_in_nano() -> u128 {
    match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
        Ok(n) => n.as_nanos(),
        Err(_) => 0,
    }
}
