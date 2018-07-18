extern crate colored;

use std::borrow::Borrow;
use std::time::Duration;

use failure::Fail;
use futures::Future;
use self::colored::*;
use telegram_bot::{
    Error as TelegramError,
    prelude::*,
    types::{Message, ParseMode},
};

use state::State;

/// Print the given error in a proper format for the user,
/// with it's causes.
pub fn _print_error<E: Fail>(err: impl Borrow<E>) {
    // Report each printable error, count them
    let count = err.borrow()
        .causes()
        .map(|err| format!("{}", err))
        .filter(|err| !err.is_empty())
        .enumerate()
        .map(|(i, err)| if i == 0 {
            eprintln!("{} {}", _highlight_error("error:"), err);
        } else {
            eprintln!("{} {}", _highlight_error("caused by:"), err);
        })
        .count();

    // Fall back to a basic message
    if count == 0 {
        eprintln!("{} {}", _highlight_error("error:"), "an undefined error occurred");
    }
}

/// Highlight the given text with an error color.
pub fn _highlight_error(msg: &str) -> ColoredString {
    msg.red().bold()
}


/// Format the given error message in Markdown to send to the user over Telegram,
/// along with it's causes.
pub fn format_error<E: Fail>(err: impl Borrow<E>) -> String {
    // Build the base error message
    let mut msg = "Whoops! An error occurred while processing your message. 😱\n\n".to_owned();

    // Build a list of error texts
    let errors: Vec<String> = err
        .borrow()
        .causes()
        .map(|err| format!("{}", err))
        .filter(|err| !err.is_empty())
        .enumerate()
        .map(|(i, err)| if i == 0 {
            format!("*error:* _{}_", err)
        } else {
            format!("*caused by:* _{}_", err)
        })
        .collect();

    // Append the errors to the message, or fall back
    if !errors.is_empty() {
        msg += &errors.join("\n");
    } else {
        msg += "*error:* _an undefined error occurred_";
    }

    msg
}

/// Handle a message error, by sending the occurred error to the user as a reply on their
/// message along with it's causes.
// TODO: create a future for this, delay it for a second to cool down from throttling
pub fn handle_msg_error<E: Fail>(state: State, msg: Message, err: impl Borrow<E>)
    -> impl Future<Item = (), Error = TelegramError>
{
    // TODO: make this timeout configurable
    state.telegram_client()
        .send_timeout(
            msg.text_reply(format_error(err))
                .parse_mode(ParseMode::Markdown),
            Duration::from_secs(30),
        )
        .map(|_| ())
}
