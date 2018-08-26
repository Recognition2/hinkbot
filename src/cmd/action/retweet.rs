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
    types::{Message, MessageChat, MessageKind, MessageOrChannelPost, ParseMode},
};

use state::State;
use super::Action;
use super::help::build_help_list;

/// The action command name.
const CMD: &'static str = "rt";

/// Whether the action is hidden.
const HIDDEN: bool = false;

/// The action help.
const HELP: &'static str = "Retweet a message";

pub struct Retweet;

impl Retweet {
    pub fn new() -> Self {
        Retweet
    }
}

impl Action for Retweet {
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
        // Get the reply message which we should retweet
        let retweet_msg: &Message = match msg.reply_to_message {
            Some(ref msg) => match msg {
                box MessageOrChannelPost::Message(msg) => msg,
                box MessageOrChannelPost::ChannelPost(_) => return Box::new(
                    state.telegram_send(
                            msg.text_reply(format!("You can't retweet a channel post."))
                                .parse_mode(ParseMode::Markdown),
                        )
                        .map(|_| ())
                        .map_err(|err| Error::Respond(SyncFailure::new(err)))
                        .from_err()
                ),
            },
            None => return Box::new(
                state.telegram_send(
                        msg.text_reply(format!("\
                                    To retweet, you must reply to a message with the `/{}` command.\
                                ",
                                CMD,
                            ))
                            .parse_mode(ParseMode::Markdown),
                    )
                    .map(|_| ())
                    .map_err(|err| Error::Respond(SyncFailure::new(err)))
                    .from_err()
            ),
        };

        // Only text messages can be retweeted
        match &retweet_msg.kind {
            MessageKind::Text { data, .. } => {
                Box::new(
                    state.telegram_send(
                            retweet_msg.text_reply(format!("\
                                    <a href=\"tg://user?id={}\">{}</a> <b>RTs:</b> {}",
                                    msg.from.id,
                                    msg.from.first_name,
                                    data,
                                ))
                                .parse_mode(ParseMode::Html),
                        )
                        .map(|_| ())
                        .map_err(|err| Error::Respond(SyncFailure::new(err)))
                        .from_err()
                )
            },
            _ => {
                Box::new(
                    state.telegram_send(
                            msg.text_reply(format!(
                                    "Only text messages can be retweeted at this moment."
                                ))
                                .parse_mode(ParseMode::Markdown),
                        )
                        .map(|_| ())
                        .map_err(|err| Error::Respond(SyncFailure::new(err)))
                        .from_err()
                )
            },
        }
    }
}

/// A start action error.
#[derive(Debug, Fail)]
pub enum Error {
    /// An error occurred while sending a response message to the user.
    #[fail(display = "failed to send response message")]
    Respond(#[cause] SyncFailure<TelegramError>),
}
