use futures::{
    future::ok,
    Future,
};
use telegram_bot::{
    Api,
    prelude::*,
    types::{Message, ParseMode},
};

use app::{NAME, VERSION};
use super::Action;

const CMD: &'static str = "genimi";

pub struct Genimi;

impl Genimi {
    pub fn new() -> Self {
        Genimi
    }
}

impl Action for Genimi {
    fn cmd(&self) -> &'static str {
        CMD
    }

    fn invoke(&self, msg: &Message, api: &Api) -> Box<Future<Item = (), Error = ()>> {
        api.spawn(
            msg.text_reply(format!(
                "\
                    `{} v{}`\n\
                    Developed by @timvisee\n\
                    https://timvisee.com/\
                ",
                NAME,
                VERSION,
            )).parse_mode(ParseMode::Markdown),
        );
        Box::new(ok(()))
    }
}
