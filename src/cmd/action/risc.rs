use std::time::Duration;

use failure::{
    Error as FailureError,
    SyncFailure,
};
use futures::Future;
use telegram_bot::{
    Api,
    Error as TelegramError,
    prelude::*,
    types::{Message, ParseMode},
};

use app::{NAME, VERSION};
use super::Action;

/// The action command name.
const CMD: &'static str = "risc";

/// Whether the action is hidden.
const HIDDEN: bool = false;

/// The action help.
const HELP: &'static str = "RISC info";

pub struct Risc;

impl Risc {
    pub fn new() -> Self {
        Risc
    }
}

impl Action for Risc {
    fn cmd(&self) -> &'static str {
        CMD
    }

    fn hidden(&self) -> bool {
        HIDDEN
    }

    fn help(&self) -> &'static str {
        HELP
    }

    fn invoke(&self, msg: &Message, api: &Api)
        -> Box<Future<Item = (), Error = FailureError>>
    {
        // Build a future for sending the response message
        // TODO: make the timeout configurable
        let future = api.send_timeout(
                msg.text_reply(format!(
                        "\
                            `{} v{}`\n\
                            \n\
                            Developed by @timvisee\n\
                            https://timvisee.com/\n\
                            \n\
                            Source:\n\
                            https://gitlab.com/timvisee/risc-bot\
                        ",
                        NAME,
                        VERSION,
                    )).parse_mode(ParseMode::Markdown),
                Duration::from_secs(10),
            )
            .map(|_| ())
            .map_err(|err| Error::Respond(SyncFailure::new(err)))
            .from_err();

        Box::new(future)
    }
}

/// A start action error.
#[derive(Debug, Fail)]
pub enum Error {
    /// An error occurred while sending a response message to the user.
    #[fail(display = "failed to send response message")]
    Respond(#[cause] SyncFailure<TelegramError>),
}
