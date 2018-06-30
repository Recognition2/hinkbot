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

/// The action command name.
const CMD: &'static str = "ping";

/// The action help.
const HELP: &'static str = "Ping RISC";

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

    fn help(&self) -> &'static str {
        HELP
    }

    fn invoke(&self, msg: &Message, api: &Api) -> Box<Future<Item = (), Error = ()>> {
        api.spawn(msg.text_reply("Pong!"));
        Box::new(ok(()))
    }
}
