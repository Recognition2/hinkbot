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
    types::Message,
};

use super::Action;

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

    fn invoke(&self, msg: &Message, api: &Api)
        -> Box<Future<Item = (), Error = FailureError>>
    {
        // Build a message future for sending the response
        // TODO: make this time configurable
        let future = api.send_timeout(
                msg.text_reply("Pong!"),
                Duration::from_secs(10),
            )
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
