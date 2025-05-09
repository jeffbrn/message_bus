pub trait Worker : Sync + Send {
    fn check_msg(&mut self, msg: i32) -> bool;
    fn handle_msg(&mut self, msg: i32) -> f32;
}
