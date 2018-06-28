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

const CMD: &'static str = "help";

pub struct Help;

impl Help {
    pub fn new() -> Self {
        Help
    }
}

impl Action for Help {
    fn cmd(&self) -> &'static str {
        CMD
    }

    fn invoke(&self, msg: &Message, api: &Api) -> Box<Future<Item = (), Error = ()>> {
        api.spawn(
            // TODO: load the available actions dynamically
            msg.text_reply("\
                Genimi commands:\n\
                /exec - Execute a shell command\n\
                /genimi - Genimi command\n\
                /ping - Ping Genimi\n\
                /test - Test command\n\
                /help - Command help\
            "),
        );
        Box::new(ok(()))
    }
}
