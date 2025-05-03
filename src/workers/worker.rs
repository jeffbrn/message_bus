pub trait Worker : Sync + Send {
    fn check_msg(&self, msg: i32) -> bool;
    fn handle_msg(&self, msg: i32) -> f32;
}
