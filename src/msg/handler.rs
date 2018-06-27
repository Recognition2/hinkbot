use futures::{
    Future,
    future::ok,
};
use regex::Regex;
use telegram_bot::{
    Api,
    prelude::*,
    types::{Message, MessageChat, MessageKind, ParseMode},
};

use cmd::handler::Handler as CmdHandler;

lazy_static! {
    static ref CMD_REGEX: Regex = Regex::new(r"^/([a-zA-Z0-9_]+)(@.*$|$)")
        .expect("failed to compile CMD_REGEX");
}

/// The generic message handler.
/// This handler should process all incomming messages from Telegram,
/// and route them to the proper actions.
pub struct Handler;

impl Handler {
    /// Handle the given message.
    pub fn handle(msg: Message, api: &Api)
        -> Box<Future<Item = (), Error = ()>>
    {
        match &msg.kind {
            MessageKind::Text {
                ref data,
                ..
            } => {
                // Route the message to the command handler, if it's a command
                if let Some(cmd) = matches_cmd(data) {
                    // TODO: do not re-box
                    return Box::new(CmdHandler::handle(cmd, msg.clone(), api));
                }

                // Route private messages
                match &msg.chat {
                    MessageChat::Private(..) => {
                        return Self::handle_private(&msg, api);
                    },
                    _ => {},
                }

            },
            _ => {},
        }

        // TODO: Use Ok ?
        Box::new(ok(()))
    }

    /// Handle the give private/direct message.
    pub fn handle_private(msg: &Message, api: &Api)
        -> Box<Future<Item = (), Error = ()>>
    {
        api.spawn(msg.text_reply(
            format!(
                "`BLEEP BLOOP`\n`I AM A BOT`\n\n{}, direct messages are not supported yet.",
                msg.from.first_name,
            )
        ).parse_mode(ParseMode::Markdown));

        Box::new(ok(()))
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
