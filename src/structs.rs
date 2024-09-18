use std::fmt;
use serde::{Deserialize, Serialize};

pub trait ConvertVecToMessageStruct {
    fn to_struct(&self) -> Message;
}
impl ConvertVecToMessageStruct for Vec<u8> {
    fn to_struct(&self) -> Message {
        match String::from_utf8(self.clone()) {
            Ok(income) => {
                let payload_result: serde_json::Result<Message> = serde_json::from_str(&income);
                if payload_result.is_ok(){
                    return payload_result.unwrap();
                }else{
                    println!("control-4523");
                }
            },
            Err(_) =>{
                println!("control-7485");
            },
        }
        Message{
            id: 0,
            sender: String::new(),
            kind: MessageKind::Error,
            payload: Vec::new(),
        }
    }
}

impl Message {
    pub fn to_byte_array(&self) -> Vec<u8> {
        match serde_json::to_string(&self.clone()) {
            Ok(result) => {
                //println!("to_byte_array: {}",result.clone());
                result.as_bytes().to_vec()
            },
            Err(_) => {
                println!("control-0023");
                Vec::new()
            },
        }    
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Message {
    pub id:u128,
    pub sender: String,
    pub kind: MessageKind,
    pub payload: Vec<u8>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum MessageKind{
    Ok,

    // node'ların canlılık durumunu kontrol etmek için
    Ping,
    
    // node listesi ve liste hash'i
    NodeList,
    
    // dağıtılacak mesaj 
    Distribute,
    
    // hatalı mesaj veya işlem tipi
    Error,
}


#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct NodeDetails {
    pub addr: String,
    pub node_hash: String,
    pub last_access_time: u128,
    pub status: NodeStatus,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum NodeStatus{
    Online,
    Offline,
    Unknown
}

impl fmt::Display for NodeStatus {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl fmt::Display for MessageKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum EventType{
    OnNodesSynced(String),
    OnNodeOnline(String),
    OnNodeOffline(String),
    OnMessage(Message),
    OnWait(),
}
