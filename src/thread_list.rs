use std::thread;
use crate::{helper, structs::{Message, MessageKind, NodeDetails}};

// TO-DO 
// node listesi 40'dan uzun ise, parçalı gönderim yap
pub fn thread_update_node_list(
    my_node_addr:String,
    node_detail:NodeDetails,
    node_list:Vec<NodeDetails>
){
    thread::spawn(move||{
        let mut tmp_node_list=Vec::new();
        for n_info in node_list.clone().iter(){
            tmp_node_list.push(n_info.addr.clone());
        }
        let node_list_array=serde_json::to_vec(&tmp_node_list).unwrap();
        
        // node listesi dağıtılıyor...
        let _result=helper::client(
            node_detail.addr.to_string(),
            Message{
                id: helper::get_sys_time_in_nano(),
                sender: my_node_addr.clone(),
                kind: MessageKind::NodeList,
                payload: node_list_array.clone(),
            }.to_byte_array()
        );
    });
}
