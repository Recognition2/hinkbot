use failure::{
    Error as FailureError,
    SyncFailure,
};
use futures::{
    Future,
    future::ok,
};
use telegram_bot::{
    Error as TelegramError,
    prelude::*,
    types::{Message, MessageKind, ParseMode},
};

use state::State;
use super::Action;

/// The action command name.
const CMD: &'static str = "echohtml";

/// Whether the action is hidden.
const HIDDEN: bool = true;

/// The action help.
const HELP: &'static str = "Echo user input as HTML";

pub struct EchoHtml;

impl EchoHtml {
    pub fn new() -> Self {
        EchoHtml
    }
}

impl Action for EchoHtml {
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
        if let MessageKind::Text {
            ref data,
            ..
        } = &msg.kind {
            // Get the user's input
            // TODO: actually properly fetch the user input
            let input = data.splitn(2, ' ')
                .skip(1)
                .next()
                .map(|cmd| cmd.trim_left())
                .unwrap_or("")
                .to_owned();

            // Build a future for sending the response message
            let future = state.telegram_send(
                    msg.text_reply(input)
                        .parse_mode(ParseMode::Html),
                )
                .map(|_| ())
                .map_err(|err| Error::Respond(SyncFailure::new(err)))
                .from_err();

            Box::new(future)
        } else {
            Box::new(ok(()))
        }
    }
}

/// A echo HTML action error.
#[derive(Debug, Fail)]
pub enum Error {
    /// An error occurred while sending a response message to the user.
    #[fail(display = "failed to send response message")]
    Respond(#[cause] SyncFailure<TelegramError>),
}
