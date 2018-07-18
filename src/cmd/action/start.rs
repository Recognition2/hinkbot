use std::time::Duration;

use failure::{
    Error as FailureError,
    SyncFailure,
};
use futures::Future;
use telegram_bot::{
    Error as TelegramError,
    prelude::*,
    types::{Message, ParseMode},
};

use state::State;
use super::Action;

/// The action command name.
const CMD: &'static str = "start";

/// Whether the action is hidden.
const HIDDEN: bool = true;

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

    fn hidden(&self) -> bool {
        HIDDEN
    }

    fn help(&self) -> &'static str {
        HELP
    }

    fn invoke(&self, state: &State, msg: &Message)
        -> Box<Future<Item = (), Error = FailureError>>
    {
        // Build a future for sending the response start message
        let future = state.telegram_client()
            .send_timeout(
                msg.text_reply(format!("\
                            *Welcome {}!*\n\
                            \n\
                            To start using this bot, see the list of available commands by sending /help\
                        ",
                        msg.from.first_name,
                    ))
                    .parse_mode(ParseMode::Markdown),
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
