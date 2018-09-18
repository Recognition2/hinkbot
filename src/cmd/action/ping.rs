use failure::{Error as FailureError, SyncFailure};
use futures::Future;
use telegram_bot::{prelude::*, types::Message, Error as TelegramError};

use super::Action;
use state::State;

/// The action command name.
const CMD: &'static str = "ping";

/// Whether the action is hidden.
const HIDDEN: bool = false;

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

    fn hidden(&self) -> bool {
        HIDDEN
    }

    fn help(&self) -> &'static str {
        HELP
    }

    fn invoke(&self, state: &State, msg: &Message) -> Box<Future<Item = (), Error = FailureError>> {
        // Build a message future for sending the response
        let future = state
            .telegram_send(msg.text_reply("Pong!"))
            .map(|_| ())
            .map_err(|err| Error::Respond(SyncFailure::new(err)))
            .from_err();

        Box::new(future)
    }
}

/// A ping action error.
#[derive(Debug, Fail)]
pub enum Error {
    /// An error occurred while sending a response message to the user.
    #[fail(display = "failed to send response message")]
    Respond(#[cause] SyncFailure<TelegramError>),
}
