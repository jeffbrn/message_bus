use std::{sync::{atomic::{ AtomicBool, Ordering::Relaxed }, Arc}, thread, time::Duration};
use crossbeam_channel::{unbounded, Receiver};
use log::{debug, info};
use super::Worker;

pub struct MsgNode {
    thrd: Option<thread::JoinHandle<()>>,
    running: Arc<AtomicBool>,
    pub enabled: Arc<AtomicBool>
}

impl MsgNode {
    pub fn new<TMsg, TResult>(mut worker: Box<dyn Worker<TMsg, TResult>>, msg_in: Receiver<TMsg>) -> (Self, Receiver<TMsg>)
        where TMsg: Send+Clone+'static, TResult: Send+Clone+'static{
        let (tx, rx) = unbounded::<TMsg>();
        let flag = Arc::new(AtomicBool::new(true));
        let run = flag.clone();
        let enable = Arc::new(AtomicBool::new(true));
        let wrk_enable = enable.clone();
        let t = thread::spawn(move || {
            while flag.load(Relaxed) {
                let msg_wrapper = msg_in.recv_timeout(Duration::from_millis(100)).ok();
                if msg_wrapper.is_none() {
                    continue;
                }
                let msg = msg_wrapper.as_ref().unwrap();
                let msg_send = msg.clone();
                let enable = wrk_enable.load(Relaxed);
                if enable && worker.check_msg(msg) {
                    let _result = worker.handle_msg(msg);
                    info!("Worker thread received message");
                } else {
                    debug!("Passing message to next");
                    tx.send(msg_send).unwrap_or_default();
                }
            }
        });
        (Self { thrd: Some(t), running: run, enabled: enable }, rx)
    }
}

impl Drop for MsgNode {
    fn drop(&mut self) {
        self.running.store(false, Relaxed);
        if let Some(thrd) = self.thrd.take() {
            thrd.join().unwrap();
        }
    }
    
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workers::Worker;
    use std::{sync::atomic::Ordering::SeqCst, thread::sleep};
    use crate::Signal;

    static WRK_GOT_MSG: AtomicBool = AtomicBool::new(false);
    static UNHANDLED_MSG: AtomicBool = AtomicBool::new(false);
    static WRK_GOT_MSG2: AtomicBool = AtomicBool::new(false);
    static UNHANDLED_MSG2: AtomicBool = AtomicBool::new(false);

    struct TestWorker {
        signal: Signal
    }
    impl TestWorker {
        fn new() -> Self {
            Self {
                signal: Signal::new(),
            }
        }
    }
    impl Worker<i32, f32> for TestWorker {
        fn check_msg(&mut self, msg: &i32) -> bool {
            msg % 2 == 0
        }

        fn handle_msg(&mut self, msg: &i32) -> f32 {
            println!("TestWorker handling message: {}", msg);
            WRK_GOT_MSG.store(true, SeqCst);
            self.signal.notify();
            *msg as f32 * 2.0
        }
    }

    #[test]
    fn test_msg_node() {
        WRK_GOT_MSG.store(false, SeqCst);
        let (tx, rx) = unbounded();
        let worker = Box::new(TestWorker::new());
        let mut sig = worker.signal.clone();
        let (node, unhandled) = MsgNode::new(worker, rx);
        assert_eq!(node.running.load(Relaxed), true);
        let thrd = thread::spawn(move || {
            println!("Listening for unhandled");
            let msg = unhandled.recv().unwrap();
            println!("Got unhandled: {}", msg);
            assert_eq!(msg, 15);
            UNHANDLED_MSG.store(true, SeqCst);
        });
        tx.send(10).unwrap();
        sig.wait();
        assert_eq!(WRK_GOT_MSG.load(SeqCst), true);
        WRK_GOT_MSG.store(false, SeqCst);
        assert_eq!(UNHANDLED_MSG.load(SeqCst), false);
        tx.send(15).unwrap();
        sleep(Duration::from_millis(500));
        assert_eq!(WRK_GOT_MSG.load(SeqCst), false);
        thrd.join().unwrap();
        assert_eq!(UNHANDLED_MSG.load(SeqCst), true);
        drop(node);
    }

    #[derive(Debug,Clone)]
    struct TestMsg {
        id: u32,
        tstamp: f32,
        mode: String,
        xyz: [f64; 3],
    }
    #[derive(Debug,Clone)]
    struct TestResult {
        id: u32,
        tstamp: f32,
        mode: String,
        meas: [f64; 3],
    }
    struct TestWorker2 {
        signal: Signal
    }
    impl TestWorker2 {
        fn new() -> Self {
            Self {
                signal: Signal::new(),
            }
        }
    }
    impl Worker<TestMsg, TestResult> for TestWorker2 {
        fn check_msg(&mut self, msg: &TestMsg) -> bool {
            println!("Checking message: {}", msg.id);
            msg.id % 2 == 0
        }

        fn handle_msg(&mut self, msg: &TestMsg) -> TestResult {
            println!("TestWorker handling message: {}", msg.id);
            WRK_GOT_MSG2.store(true, SeqCst);
            self.signal.notify();
            TestResult {
                id: msg.id,
                tstamp: msg.tstamp,
                mode: msg.mode.clone(),
                meas: [msg.xyz[0] * 2.0, msg.xyz[1] * 2.0, msg.xyz[2] * 2.0],
            }
        }
    }

    #[test]
    fn test_msg_node_complex() {
        WRK_GOT_MSG2.store(false, SeqCst);
        let (tx, rx) = unbounded();
        let worker = Box::new(TestWorker2::new());
        let mut sig = worker.signal.clone();
        let (node, unhandled) = MsgNode::new(worker, rx);
        assert_eq!(node.running.load(Relaxed), true);
        let thrd = thread::spawn(move || {
            println!("Listening for unhandled");
            let msg = unhandled.recv().unwrap();
            println!("Got unhandled: {:?}", msg);
            assert_eq!(msg.id, 15);
            UNHANDLED_MSG2.store(true, SeqCst);
        });
        tx.send(TestMsg {id: 10, tstamp: 1.1, mode: "12".to_string(), xyz: [1.1, 1.2, 1.3] } ).unwrap();
        sig.wait();
        assert_eq!(WRK_GOT_MSG2.load(SeqCst), true);
        WRK_GOT_MSG2.store(false, SeqCst);
        assert_eq!(UNHANDLED_MSG2.load(SeqCst), false);
        tx.send(TestMsg {id: 15, tstamp: 1.1, mode: "12".to_string(), xyz: [1.1, 1.2, 1.3] }).unwrap();
        sleep(Duration::from_millis(500));
        assert_eq!(WRK_GOT_MSG2.load(SeqCst), false);
        thrd.join().unwrap();
        assert_eq!(UNHANDLED_MSG2.load(SeqCst), true);
        drop(node);
    }

}