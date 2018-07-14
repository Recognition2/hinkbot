use std::time::Duration;

use failure::SyncFailure;
use futures::{
    Future,
    future::ok,
};
use telegram_bot::{
    Error as TelegramError,
    prelude::*,
    types::{Message, MessageChat, MessageKind, ParseMode},
};

use cmd::handler::{
    Error as CmdHandlerError,
    Handler as CmdHandler,
    matches_cmd,
};
use state::State;

/// The generic message handler.
/// This handler should process all incomming messages from Telegram,
/// and route them to the proper actions.
pub struct Handler;

impl Handler {
    /// Handle the given message.
    pub fn handle(state: &State, msg: Message)
        -> Box<Future<Item = (), Error = Error>>
    {
        match &msg.kind {
            MessageKind::Text {
                ref data,
                ..
            } => {
                // Log all incomming text messages
                println!(
                    "MSG <{}>@{}: {}",
                    &msg.from.first_name,
                    &msg.chat.id(),
                    data,
                );

                // Update the message stats
                state.stats().increase_messages(msg.chat.id(), msg.from.id);

                // Route the message to the command handler, if it's a command
                if let Some(cmd) = matches_cmd(data) {
                    return Box::new(
                        CmdHandler::handle(state, cmd, msg.clone()).from_err(),
                    );
                }

                // Route private messages
                match &msg.chat {
                    MessageChat::Private(..) =>
                        return Box::new(
                            Self::handle_private(state, &msg),
                        ),
                    _ => {},
                }
            },
            _ => {},
        }

        Box::new(ok(()))
    }

    /// Handle the give private/direct message.
    pub fn handle_private(state: &State, msg: &Message)
        -> impl Future<Item = (), Error = Error>
    {
        // Send a message to the user
        // TODO: make timeout configurable
        state.telegram_client()
            .send_timeout(
                msg.text_reply(format!(
                        "`BLEEP BLOOP`\n`I AM A BOT`\n\n{}, direct messages are not supported yet.",
                        msg.from.first_name,
                    ))
                    .parse_mode(ParseMode::Markdown),
                Duration::from_secs(10),
            )
            .map(|_| ())
            .map_err(|err| Error::HandlePrivate(SyncFailure::new(err)))
    }
}

/// A message handler error.
#[derive(Debug, Fail)]
pub enum Error {
    /// An error occurred while processing a command.
    #[fail(display = "failed to process command message")]
    HandleCmd(#[cause] CmdHandlerError),

    /// An error occurred while processing a private message.
    #[fail(display = "failed to process private message")]
    HandlePrivate(#[cause] SyncFailure<TelegramError>),
}

/// Convert a command handler error into a message handling error.
impl From<CmdHandlerError> for Error {
    fn from(err: CmdHandlerError) -> Error {
        Error::HandleCmd(err)
    }
}
