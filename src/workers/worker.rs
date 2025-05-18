pub trait Worker<TMsg, TResult> : Sync + Send where TMsg: Send, TResult: Send {
    fn check_msg(&mut self, msg: &TMsg) -> bool;
    fn handle_msg(&mut self, msg: &TMsg) -> TResult;
}
