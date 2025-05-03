use mbus::Worker;
use mbus::MessageBusSeq;
//use mbus::workers::MsgNode;

struct DoubleWorker {}
impl Worker for DoubleWorker {
    fn check_msg(&self, msg: i32) -> bool {
        msg % 2 == 0
    }

    fn handle_msg(&self, msg: i32) -> f32 {
        println!("=============================================");
        println!("DoubleWorker handling message: {}", msg);
        println!("=============================================");
            msg as f32 * 2.0
    }
}

fn main() {
    println!("Hello, world!");
    let wrk1 = Box::new(DoubleWorker {});
    let mbus = MessageBusSeq::new();
}
