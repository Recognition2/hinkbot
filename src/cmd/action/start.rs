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

/// The action command name.
const CMD: &'static str = "start";

/// The action help.
const HELP: &'static str = "Start using RISC";

pub struct Start;

impl Start {
    pub fn new() -> Self {
        Start
    }
}

impl Action for Start {
    fn cmd(&self) -> &'static str {
        CMD
    }

    fn help(&self) -> &'static str {
        HELP
    }

    fn invoke(&self, msg: &Message, api: &Api) -> Box<Future<Item = (), Error = ()>> {
        // Send the help message
        api.spawn(
            msg.text_reply(format!("\
                    *Welcome {}!*\n\
                    \n
                    To start using this bot, see the list of available commands by typing /help\
                ",
                msg.from.first_name,
            ))
            .parse_mode(ParseMode::Markdown),
        );

        Box::new(ok(()))
    }
}
