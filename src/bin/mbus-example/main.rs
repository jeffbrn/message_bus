use std::thread::sleep;
use std::time::Duration;

use mbus::Worker;
use mbus::MessageBusSeq;
use log::{debug, info};

struct DoubleWorker {}
impl Worker<i32, f32> for DoubleWorker {
    fn check_msg(&mut self, msg: &i32) -> bool {
        msg % 2 == 0
    }

    fn handle_msg(&mut self, msg: &i32) -> f32 {
        println!("=============================================");
        println!("DoubleWorker handling message: {}", msg);
        println!("=============================================");
            *msg as f32 * 2.0
    }
}

struct TripleWorker {}
impl Worker<i32, f32> for TripleWorker {
    fn check_msg(&mut self, msg: &i32) -> bool {
        msg % 3 == 0
    }

    fn handle_msg(&mut self, msg: &i32) -> f32 {
        println!("=============================================");
        println!("TripleWorker handling message: {}", msg);
        println!("=============================================");
            *msg as f32 * 3.0
    }
}

fn unhandled(msg: &i32) {
    println!("=============================================");
    println!("Unhandled message: {}", msg);
    println!("=============================================");
}

fn main() {
    simple_logger::init_with_level(log::Level::Trace).unwrap();
    let mut mbus = MessageBusSeq::new::<i32>();
    debug!("MessageBus created");
    mbus.set_unhandled(unhandled);
    mbus.publish(10);
    mbus.add_worker(Box::new(DoubleWorker {}));
    mbus.publish(20);
    mbus.publish(15);
    sleep(Duration::from_millis(200));
    mbus.set_enabled(0, false);
    mbus.add_worker(Box::new(TripleWorker {}));
    mbus.publish(20);
    mbus.publish(15);
    println!("# of workers = {}", mbus.len());
    info!("\nDone!");
}
