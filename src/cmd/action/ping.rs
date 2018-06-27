use futures::{
    future::ok,
    Future,
};
use telegram_bot::{
    Api,
    prelude::*,
    types::Message,
};

use super::Action;

const CMD: &'static str = "ping";

pub struct Ping;

impl Ping {
    pub fn new() -> Self {
        Ping
    }
}

impl Action for Ping {
    fn cmd(&self) -> &'static str {
        CMD
    }

    fn invoke(&self, msg: &Message, api: &Api) -> Box<Future<Item = (), Error = ()>> {
        api.spawn(msg.text_reply("Pong!"));
        Box::new(ok(()))
    }
}
