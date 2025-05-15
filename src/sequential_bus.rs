use std::sync::Condvar;
use std::thread;
use std::time::Duration;
use std::sync::{Arc, atomic::{AtomicBool, Ordering::Relaxed}, Mutex, RwLock};
use crossbeam_channel::{unbounded, Receiver, Sender};
use log::{debug, trace};
use crate::workers::{Worker, msg_node::MsgNode};

//TODO: templates
//TODO: results

pub type MsgBusHandler = fn(&i32) -> ();

pub struct MessageBusSeq {
    data: Arc<RwLock<MsgBusState>>,
    input_msg: Sender<i32>,
    thrd: Option<thread::JoinHandle<()>>,
    workers: Vec<MsgNode>,
}

struct MsgBusState {
    unhandled_msg: Receiver<i32>,
    unhandled_msg_handler: Option<MsgBusHandler>,
    running: AtomicBool,
    pub num_msgs: usize,
    pub num_unhandled: usize,
}

impl MessageBusSeq {

    pub fn new() -> Self {
        let (msg_tx, msg_rx) = unbounded();
        let mut retval = Self {
            data: Arc::new(RwLock::new(MsgBusState {
                unhandled_msg: msg_rx,
                unhandled_msg_handler: None,
                running: AtomicBool::new(true),
                num_msgs: 0,
                num_unhandled: 0,
            })),
            input_msg: msg_tx,
            thrd: None,
            workers: vec![],
        };
        let (thrd_signal, thrd) = MessageBusSeq::init_mb_daemon(retval.data.clone());
        retval.thrd = Some(thrd);
        {
            trace!("Waiting for worker thread to start");
            let (lck, cvar) = &*thrd_signal;
            let lck = lck.lock().unwrap();
            drop(cvar.wait(lck).unwrap());
            debug!("Worker thread started");
        }
        retval
    }

    /// publish a message onto the bus
    pub fn publish(&self, msg: i32) {
        // update the message count
        {
            let mut data = self.data.write().unwrap();
            data.num_msgs += 1;
        }
        trace!("Publishing message: {}", msg);
        // put message onto the bus
        self.input_msg.send(msg).unwrap();
    }

    /// set a handler to receive unprocessed messages
    pub fn set_unhandled(&mut self, handler: MsgBusHandler) {
        self.data.write().unwrap().unhandled_msg_handler = Some(handler);
        trace!("Unhandled message handler set");
    }

    pub fn add_worker(&mut self, worker: Box<dyn Worker>) {
        let mut data =self.data.write().unwrap();
        let (node, msg_out) = MsgNode::new(worker, data.unhandled_msg.clone());
        self.workers.push(node);
        data.unhandled_msg = msg_out;
    }

    pub fn len(&self) -> usize {
        self.workers.len()
    }

    pub fn is_empty(&self) -> bool {
        self.workers.is_empty()
    }

    pub fn get_num_msgs(&self) -> usize {
        self.data.read().unwrap().num_msgs
    }
    pub fn get_num_unhandled(&self) -> usize {
        self.data.read().unwrap().num_unhandled
    }

    pub fn set_enabled(&mut self, idx: usize, enabled: bool) {
        if idx < self.workers.len() {
            self.workers[idx].enabled.store(enabled, Relaxed);
        }
    }

    /// Initialize the message bus daemon thread
    fn init_mb_daemon(thrd_data: Arc<RwLock<MsgBusState>>) -> (Arc<(Mutex<()>, Condvar)>, thread::JoinHandle<()>) {
        // setup  way for the deaomon to signal the main thread that it is ready to recive messages
        let startup = Arc::new((Mutex::new(()), Condvar::new()));
        let signal = startup.clone();
        // spawn the daemon thread
        let t = thread::spawn(move || {
            trace!("MsgBus thread starting");
            {
                startup.1.notify_all();
            }
            // keep running until  the stop flag is set
            while thrd_data.read().unwrap().running.load(Relaxed) {
                let queue = thrd_data.read().unwrap().unhandled_msg.clone();
                // check for any unhandled messages
                let msg = queue.recv_timeout(Duration::from_millis(100)).ok();
                if msg.is_none() {
                    // no message, so check if we are still running
                    continue;
                }
                // process the unhandled message
                let msg = msg.unwrap();
                debug!("MsgBus thread received message: {}", msg);
                {
                    let mut data = thrd_data.write().unwrap();
                    // update the message count
                    data.num_unhandled += 1;
                }
                // check if we have a handler for the message
                let handler = thrd_data.read().unwrap().unhandled_msg_handler;
                if let Some(handler) = handler {
                    // call the handler
                    handler(&msg);
                }
            }
            debug!("MsgBus thread ending");
        });
        (signal, t)
    }

}

impl Drop for MessageBusSeq {
    fn drop(&mut self) {
        // stop the thread
        self.data.write().unwrap().running.store(false, Relaxed);
        if let Some(thrd) = self.thrd.take() {
            thrd.join().unwrap();
        }
    }
}

impl Default for MessageBusSeq {
    fn default() -> Self {
        Self::new()
    }
}
#[cfg(test)]
mod tests {
    use std::thread::sleep;

    use super::*;
    use crate::workers::Worker;
    use crate::Signal;

    struct TestWorker {
        divisor: i32,
        signal: Signal
    }
    impl TestWorker {
        fn new(divisor: i32) -> Self {
            Self {
                divisor,
                signal: Signal::new(),
            }
        }
    }
    impl Worker for TestWorker {
        fn check_msg(&mut self, msg: i32) -> bool {
            msg % self.divisor == 0
        }

        fn handle_msg(&mut self, msg: i32) -> f32 {
            self.signal.notify();
            msg as f32 * 2.0
        }
    }

    fn test_unhandled(msg: &i32) {
        println!("=============================================");
        println!("Unhandled message: {}", msg);
        println!("=============================================");
    }

    #[test]
    fn test_no_workers() {
        let mut mbus = MessageBusSeq::new();
        mbus.set_unhandled(test_unhandled);
        assert_eq!(mbus.len(), 0);
        mbus.publish(10);
        sleep(Duration::from_millis(500));
        assert_eq!(mbus.get_num_msgs(), 1);
        assert_eq!(mbus.get_num_unhandled(), 1);
    }

    #[test]
    fn test_single_worker() {
        let mut mbus = MessageBusSeq::new();
        mbus.set_unhandled(test_unhandled);
        let wrk = TestWorker::new(2);
        let mut sig = wrk.signal.clone();
        mbus.add_worker(Box::new(wrk));
        sleep(Duration::from_millis(500));
        mbus.publish(20);
        sig.wait();
        assert_eq!(mbus.get_num_msgs(), 1);
        assert_eq!(mbus.get_num_unhandled(), 0);
        mbus.publish(15);
        sleep(Duration::from_millis(500));
        assert_eq!(mbus.get_num_msgs(), 2);
        assert_eq!(mbus.get_num_unhandled(), 1);
    }

    #[test]
    fn test_multiple_workers() {
        let mut mbus = MessageBusSeq::new();
        mbus.set_unhandled(test_unhandled);
        let wrk1 = TestWorker::new(2);
        let mut sig1 = wrk1.signal.clone();
        mbus.add_worker(Box::new(wrk1));
        let wrk2 = TestWorker::new(3);
        let mut sig2 = wrk2.signal.clone();
        mbus.add_worker(Box::new(wrk2));
        sleep(Duration::from_millis(500));
        mbus.publish(20);
        sig1.wait();
        assert_eq!(mbus.get_num_msgs(), 1);
        assert_eq!(mbus.get_num_unhandled(), 0);
        mbus.publish(15);
        sig2.wait();
        assert_eq!(mbus.get_num_msgs(), 2);
        assert_eq!(mbus.get_num_unhandled(), 0);
        mbus.publish(5);
        sleep(Duration::from_millis(500));
        assert_eq!(mbus.get_num_msgs(), 3);
        assert_eq!(mbus.get_num_unhandled(), 1);
    }
}