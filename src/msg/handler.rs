use std::time::Duration;

use failure::SyncFailure;
use futures::{
    Future,
    future::ok,
};
use regex::Regex;
use telegram_bot::{
    Api,
    Error as TelegramError,
    prelude::*,
    types::{Message, MessageChat, MessageKind, ParseMode},
};

use cmd::handler::{
    Error as CmdHandlerError,
    Handler as CmdHandler,
};

lazy_static! {
    /// A regex for matching messages that contain a command.
    static ref CMD_REGEX: Regex = Regex::new(r"^/([a-zA-Z0-9_]+)(.*$|$)")
        .expect("failed to compile CMD_REGEX");
}

/// The generic message handler.
/// This handler should process all incomming messages from Telegram,
/// and route them to the proper actions.
pub struct Handler;

impl Handler {
    /// Handle the given message.
    pub fn handle(msg: Message, api: &Api)
        -> Box<Future<Item = (), Error = Error>>
    {
        match &msg.kind {
            MessageKind::Text {
                ref data,
                ..
            } => {
                // Route the message to the command handler, if it's a command
                if let Some(cmd) = matches_cmd(data) {
                    return Box::new(
                        CmdHandler::handle(cmd, msg.clone(), api).from_err(),
                    );
                }

                // Route private messages
                match &msg.chat {
                    MessageChat::Private(..) =>
                        return Box::new(
                            Self::handle_private(&msg, api),
                        ),
                    _ => {},
                }
            },
            _ => {},
        }

        Box::new(ok(()))
    }

    /// Handle the give private/direct message.
    pub fn handle_private(msg: &Message, api: &Api)
        -> impl Future<Item = (), Error = Error>
    {
        // Send a message to the user
        // TODO: make timeout configurable
        api.send_timeout(
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

/// Test whehter the given message is recognized as a command.
///
/// The actual command name is returned if it is, `None` otherwise.
// TODO: if a target bot is given with `/cmd@bot`, ensure it's username is matching
fn matches_cmd(msg: &str) -> Option<&str> {
    if let Some(groups) = CMD_REGEX.captures(msg.trim()) {
        Some(groups.get(1).unwrap().as_str())
    } else {
        None
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
