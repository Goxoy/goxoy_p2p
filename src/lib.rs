use std::{ net::TcpListener, sync::{Arc, Mutex}, thread};
use colored::Colorize;
use config::Config;
use serde::{Deserialize, Serialize};
use structs::{Message, MessageKind, NodeDetails, NodeStatus};
use worker::ThreadPool;

mod thread_list;
mod helper;
mod handle_connection;
mod config;
mod structs;
mod worker;

pub struct MessageConfig {
    ping_time: u128
}
pub struct MessagePool {
    pub my_addr: String,
    debug: bool,
    hard_config: MessageConfig,
    node_hash: Arc<Mutex<String>>,
    node_list_synced: Arc<Mutex<String>>,
    node_hash_updated: Arc<Mutex<bool>>,
    node_status_change: Arc<Mutex<Vec<(String,NodeStatus)>>>,
    node_list: Arc<Mutex<Vec<NodeDetails>>>,
    msg_list: Arc<Mutex<Vec<Message>>>,
}

impl MessagePool {
    pub fn new() -> Self {
        MessagePool {
            my_addr: String::new(),
            debug: true,
            hard_config: MessageConfig{
                ping_time: 250,
            },
            node_list_synced: Arc::new(Mutex::new(String::new())),
            node_hash_updated: Arc::new(Mutex::new(true)),
            node_hash: Arc::new(Mutex::new(String::new())),
            node_status_change: Arc::new(Mutex::new(Vec::new())),
            node_list: Arc::new(Mutex::new(Vec::new())),
            msg_list: Arc::new(Mutex::new(Vec::new())),
        }
    }
    
    pub fn start(&mut self,config_file_name:Option<String>){
        let conf=Config::new(config_file_name);
        self.debug=conf.debug;

        self.my_addr=conf.addr.clone();
        self.add_node_to_list(conf.addr.clone());
        self.add_node_to_lists(conf.boostrap.clone());

        println!("{}    => {}","STARTING NODE".bright_cyan(), self.my_addr.clone().to_string().bright_cyan());

        self.update_node_hash();
        if self.thread_socket()==true{
            self.thread_ping();
        }else{
            panic!("TCP Could Not started");
        }
    }

    fn process_message(&mut self,income_msg:Message){
        match income_msg.kind.clone() {
            MessageKind::Ping => {
                match String::from_utf8(income_msg.payload.clone()) {
                    Ok(income_node_list_hash) => {
                        self.add_node_to_list(income_msg.sender.clone());

                        let new_node_hash=helper::calculate_node_list_hash(self.node_list.lock().unwrap().clone());
                        if self.node_hash.lock().unwrap().eq(&new_node_hash)==false{
                            *self.node_hash.lock().unwrap()=new_node_hash.clone();
                        }

                        for item in self.node_list.lock().unwrap().iter_mut(){
                            if item.addr.eq(&income_msg.sender.clone()){
                                item.node_hash = income_node_list_hash.clone();
                            }
                        }
                        *self.node_hash.lock().unwrap() = helper::calculate_node_list_hash(self.node_list.lock().unwrap().clone());
                    },
                    Err(_) => {},
                };
            },
            MessageKind::NodeList => {
                match String::from_utf8(income_msg.payload.clone()) {
                    Ok(income) => {
                        let payload_result: serde_json::Result<Vec<String>> = serde_json::from_str(&income);
                        if payload_result.is_ok(){
                            let payload_result=payload_result.unwrap();
                            if self.add_node_to_lists(payload_result.clone())==true{
                                *self.node_hash.lock().unwrap() = helper::calculate_node_list_hash(self.node_list.lock().unwrap().clone());
                            }
                        }
                    },
                    Err(_) => {},
                }
            },
            MessageKind::Error => {
                // println!("Error")
            },
            MessageKind::Ok => {
                //
            },
            MessageKind::Distribute => {
                if self.debug==true{
                    println!("distribute message");
                }
            },
        }
    }

    pub fn get_message(&mut self)->Message{
        let income_msg=self.msg_list.lock().unwrap()[0].clone();
        self.dispose_message();
        self.process_message(income_msg.clone());
        income_msg
    }
    
    pub fn dispose_message(&mut self){
        self.msg_list.lock().unwrap().remove(0);
    }

    pub fn distribute(&mut self,payload:Vec<u8>){
        for n_info in self.node_list.lock().unwrap().iter(){
            if n_info.addr.eq(&self.my_addr.clone())==false{
                let msg_struct=Message{
                    id: helper::get_sys_time_in_nano(),
                    sender: self.my_addr.clone(),
                    kind: MessageKind::Distribute,
                    payload: payload.clone(),
                };
                let _result=helper::client(
                    n_info.addr.to_string(),
                    msg_struct.to_byte_array()
                );
            }
        }
    }

    pub fn on_event(&mut self)->EventType{

        //önce node durumları değişenler geri gönderiliyor..
        let ( node_addr, node_status )=self.status_changed();
        if node_status==NodeStatus::Online {
            return EventType::OnNodeOnline(node_addr);
        }
        if node_status==NodeStatus::Offline {
            return EventType::OnNodeOffline(node_addr);
        }
        
        if self.node_hash_updated.lock().unwrap().clone()==true{
            *self.node_hash_updated.lock().unwrap()=false;
            let current_node_list_hash=self.node_list_synced.lock().unwrap().clone();
            if current_node_list_hash.len()>0{
                return EventType::OnNodesSynced(self.node_list_synced.lock().unwrap().clone());
            }
        }

        let msg_kind=self.select();
        if msg_kind==MessageKind::Distribute{
            return EventType::OnMessage(self.get_message());
        }
        
        if msg_kind!=MessageKind::Error{
            let _income_msg=self.get_message();
        }
        
        return EventType::OnWait();

    }
    pub fn status_changed(&mut self)->(String,NodeStatus){
        if self.node_status_change.lock().unwrap().len()==0{
            return ( String::new(), NodeStatus::Unknown );
        }

        let new_node_list_hash = helper::calculate_node_list_hash(self.node_list.lock().unwrap().clone());
        for n_info in self.node_list.lock().unwrap().iter_mut(){
            if n_info.addr.eq(&self.my_addr.clone()){
                n_info.node_hash = new_node_list_hash.clone();
                *self.node_hash.lock().unwrap() = new_node_list_hash.clone();
            }
        }
    
        let (node_addr,node_status) = self.node_status_change.lock().unwrap()[0].clone();
        if node_status==NodeStatus::Online || node_status==NodeStatus::Offline{
            self.node_status_change.lock().unwrap().remove(0);
            return ( node_addr , node_status );
        }else{
            return ( String::new(), NodeStatus::Unknown );
        }
    }

    pub fn select(&mut self)->MessageKind{
        if self.msg_list.lock().unwrap().len()>0{
            let income_msg=self.msg_list.lock().unwrap()[0].clone();
            if income_msg.kind==MessageKind::Distribute{
                return MessageKind::Distribute;
            }
            if income_msg.kind==MessageKind::NodeList{
                return MessageKind::NodeList;
            }
            if income_msg.kind==MessageKind::Ping{
                return MessageKind::Ping;
            }
        }
        MessageKind::Error
    }

    fn thread_ping(&self){
        let my_node_addr=self.my_addr.clone();
        let node_list=self.node_list.clone();
        let node_status_change=self.node_status_change.clone();
        let current_node_hash=self.node_hash.clone();
        let my_node_hash=self.node_hash.clone();
        let debug_mode=self.debug.clone();
        let node_hash_updated=self.node_hash_updated.clone();
        let node_list_synced=self.node_list_synced.clone();
        let hard_config_ping_time=self.hard_config.ping_time;
        thread::spawn(move||{ 
        let mut all_node_list_changed=true;
        let mut ping_time_diff= 0;
        loop{
            let mut next_ping_time_diff=hard_config_ping_time;
            let mut update_node_hash_value=false;
            let mut update_node_status=Vec::new();
            let mut update_time=Vec::new();
            let mut move_to_offline_node=usize::MAX;
            for (n_index,n_info) in node_list.lock().unwrap().clone().iter().enumerate(){
                if move_to_offline_node==usize::MAX && my_node_addr.eq(&n_info.addr.clone()) == false {
                    let send_ping_to_node= match n_info.status{
                        NodeStatus::Online => {
                            let curr_millis=helper::get_sys_time_in_millis();
                            let time_diff=n_info.last_access_time.abs_diff(curr_millis);
                            if time_diff>ping_time_diff{
                                true
                            }else{
                                false
                            }
                        },
                        NodeStatus::Offline => {
                            let current_time=helper::get_sys_time_in_secs();
                            if current_time.abs_diff(n_info.last_access_time)>10{
                                // TO-DO 
                                // eğer kontrol edildiği zaman yine offline ise,
                                // bu listeden çıkartıp offline listesine al
                                if debug_mode==true{
                                    println!("{}   => {}",n_info.addr.clone(),"testing offline");
                                }
                                next_ping_time_diff=0;
                                true
                            }else{
                                false
                            }
                        },
                        NodeStatus::Unknown => {
                            next_ping_time_diff=0;
                            if debug_mode==true{
                                println!("{}   => {}",n_info.addr.clone(),"testing unknown");
                            }    
                            true
                        },
                    };
                    
                    if send_ping_to_node == true {
                        let sending_data=Message{
                            id: helper::get_sys_time_in_nano(),
                            sender: my_node_addr.clone(),
                            kind: MessageKind::Ping,
                            payload: current_node_hash.lock().unwrap().as_bytes().to_vec().clone(),
                        }.to_byte_array();
                        let result=helper::client(
                            n_info.addr.to_string(),
                            sending_data
                        );
                        if result.kind == MessageKind::Ok {
                            update_time.push((n_info.addr.clone(),helper::get_sys_time_in_millis()));
                            if n_info.status != NodeStatus::Online {
                                update_node_status.push((n_info.addr.clone(),NodeStatus::Online));
                                node_status_change.lock().unwrap().push(
                                    (n_info.addr.clone(),NodeStatus::Online)
                                );
                                update_node_hash_value=true;
                            }
                        }else{
                            if result.id == 5 {
                                if n_info.status == NodeStatus::Offline{
                                    move_to_offline_node=n_index.clone();
                                    update_node_hash_value=true;
                                }
                                if n_info.status == NodeStatus::Unknown{
                                    update_node_status.push((n_info.addr.clone(),NodeStatus::Offline));
                                    node_status_change.lock().unwrap().push(
                                        (n_info.addr.clone(),NodeStatus::Unknown)
                                    );
                                    update_node_hash_value=true;
                                }
                                if n_info.status == NodeStatus::Online {
                                    update_node_status.push((n_info.addr.clone(),NodeStatus::Offline));
                                    node_status_change.lock().unwrap().push(
                                        (n_info.addr.clone(),NodeStatus::Offline)
                                    );
                                    update_node_hash_value=true;
                                }
                            }else{
                                if result.id != 9 {
                                    if debug_mode==true{
                                        println!("result: {:?}",result);
                                    }
                                }
                            }
                        }
                    }
                }
            }

            if move_to_offline_node!=usize::MAX{
                node_list.lock().unwrap().remove(move_to_offline_node);
                all_node_list_changed=true;
            }else{
                for (n_addr,n_time) in update_time.iter(){
                    for n_info in node_list.lock().unwrap().iter_mut(){
                        if n_info.addr.eq(n_addr){
                            n_info.last_access_time = n_time.clone();
                        }
                    }
                }
                
                for (o_node,n_status) in update_node_status.iter(){
                    for n_info in node_list.lock().unwrap().iter_mut(){
                        if n_info.addr.eq(o_node){
                            n_info.status=n_status.clone();
                        }
                    }
                }
    
                let tmp_node_list=node_list.lock().unwrap().clone();
                for n_info in tmp_node_list.iter(){
                    if my_node_addr.eq(&n_info.addr.clone())==false{
                        if my_node_hash.lock().unwrap().eq(&n_info.node_hash)==false{
                            if n_info.status==NodeStatus::Online{
                                //println!("sync with        => {}",n_info.addr.clone());
                                thread_list::thread_update_node_list(
                                    my_node_addr.clone(),
                                    n_info.clone(),
                                    node_list.lock().unwrap().clone()
                                );
                            }
                        }
                    }
                }
    
                if helper::control_nodes_hash(node_list.lock().unwrap().clone()){
                    if all_node_list_changed==true {
                        all_node_list_changed=false;
                        let node_hash_cloned=my_node_hash.lock().unwrap().clone();
                        let current_list_hash=node_list_synced.lock().unwrap().clone();
                        if current_list_hash.eq(&node_hash_cloned.clone())==false{
                            *node_list_synced.lock().unwrap()=node_hash_cloned.clone();
                            *node_hash_updated.lock().unwrap()=true;
                        }
                    }
                }else{
                    all_node_list_changed=true;
                }    
            }

            if update_node_hash_value==true{
                let new_node_list_hash = helper::calculate_node_list_hash(node_list.lock().unwrap().clone());
                if current_node_hash.lock().unwrap().clone().eq(&new_node_list_hash)==false{
                    all_node_list_changed=true;
                }
                *current_node_hash.lock().unwrap() = new_node_list_hash.clone();
                for n_info in node_list.lock().unwrap().iter_mut(){
                    if my_node_addr.eq(&n_info.addr.clone()) {
                        if n_info.node_hash.eq(&new_node_list_hash.clone())==false{
                            all_node_list_changed=true;
                            n_info.node_hash = new_node_list_hash.clone();
                        }
                    }
                }
            }
        
            ping_time_diff= next_ping_time_diff;

            if ping_time_diff>0 {
                //thread::sleep(Duration::from_millis(50));
            }
        }});
    }
    
    fn thread_socket(&self)->bool{
        let listener = TcpListener::bind(self.my_addr.clone());
        if listener.is_err(){
            return false;
        }
        let listener=listener.unwrap();
        let pool = ThreadPool::new(4);
        let msg_list_cloned=self.msg_list.clone();
        let my_addr=self.my_addr.clone();
        thread::spawn(move||loop{
            for stream in listener.incoming() {
                let stream = stream.unwrap();
                let msg_list_cloned_inner=msg_list_cloned.clone();
                let my_addr_cloned=my_addr.clone();
                pool.execute(move || {
                    handle_connection::handle_connection(
                        my_addr_cloned,
                        stream,
                        msg_list_cloned_inner.clone()
                    );
                });
            }
        });
        return true;
    }

    fn update_node_hash(&mut self){
        let my_node_hash = helper::calculate_node_list_hash(self.node_list.lock().unwrap().clone());
        *self.node_hash.lock().unwrap() = my_node_hash.clone();
        for n_info in self.node_list.lock().unwrap().iter_mut(){
            if self.my_addr.eq(&n_info.addr.clone()) {
                n_info.node_hash = my_node_hash.clone();
            }
        }
    }

    pub fn add_node_to_lists(&mut self,node_list:Vec<String>)->bool{
        let mut updated=false;
        for node_addr in node_list.iter(){
            if self.add_node_to_list(node_addr.to_string())==true{
                updated=true;
            }
        }
        return updated;
    }

    pub fn add_node_to_list(&mut self,node_addr:String)->bool{
        let mut updated=false;
        if node_addr.len()==0{
            return updated;
        }
        let mut node_exist=false;
        for n_info in self.node_list.lock().unwrap().iter(){
            if node_addr.eq(&n_info.addr.clone()){
                node_exist=true;
            }
        }
        if node_exist==false{
            updated=true;
            self.node_list.lock().unwrap().push(NodeDetails{
                addr: node_addr.clone(),
                status: if node_addr.eq(&self.my_addr){
                    NodeStatus::Online
                }else{
                    NodeStatus::Unknown
                },
                last_access_time: 0,
                node_hash: String::new(),
            });
        }
        updated
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
