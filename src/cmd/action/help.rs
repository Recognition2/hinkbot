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
use super::super::handler::ACTIONS;

/// The action command name.
const CMD: &'static str = "help";

/// Whether the action is hidden.
const HIDDEN: bool = false;

/// The action help.
const HELP: &'static str = "Show help";

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

    fn hidden(&self) -> bool {
        HIDDEN
    }

    fn help(&self) -> &'static str {
        HELP
    }

    fn invoke(&self, state: &State, msg: &Message)
        -> Box<Future<Item = (), Error = FailureError>>
    {
        // Build the command list
        let mut cmds: Vec<String> = ACTIONS.iter()
            .filter(|action| !action.hidden())
            .map(|action| format!(
                "/{}: _{}_",
                action.cmd(),
                action.help(),
            ))
            .collect();
        cmds.sort();
        let cmd_list = cmds.join("\n");

        // Build a future for sending the response help message
        // TODO: make this timeout configurable
        let future = state.telegram_client()
            .send_timeout(
                msg.text_reply(format!(
                        "*RISC commands:*\n{}",
                        cmd_list,
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

/// A help action error.
#[derive(Debug, Fail)]
pub enum Error {
    /// An error occurred while sending a response message to the user.
    #[fail(display = "failed to send response message")]
    Respond(#[cause] SyncFailure<TelegramError>),
}
