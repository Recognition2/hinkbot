use futures::{
    future::ok,
    Future,
};
use telegram_bot::{
    Api,
    prelude::*,
    types::{Message, ParseMode},
};

use super::Action;

const CMD: &'static str = "test";

pub struct Test;

impl Test {
    pub fn new() -> Self {
        Test
    }
}

impl Action for Test {
    fn cmd(&self) -> &'static str {
        CMD
    }

    fn invoke(&self, msg: &Message, api: &Api) -> Box<Future<Item = (), Error = ()>> {
        api.spawn(
            msg.text_reply("<i>Jep... works on my machine!</i>")
                .parse_mode(ParseMode::Html),
        );
        Box::new(ok(()))
    }
}
