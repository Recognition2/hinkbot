use futures::{
    Future,
    future::ok,
};
use telegram_bot::{
    Api,
    types::Message,
};

use super::action::{
    Action,
    Error as ActionError,
};
use super::action::exec::Exec;
use super::action::help::Help;
use super::action::id::Id;
use super::action::ping::Ping;
use super::action::risc::Risc;
use super::action::start::Start;
use super::action::test::Test;

lazy_static! {
    /// A list of all available and invokable actions.
    /// This list includes hidden actions which may be filtered using the `.hidden()` propery.
    pub static ref ACTIONS: Vec<Box<dyn Action + Sync>> = vec![
        Box::new(Exec::new()),
        Box::new(Help::new()),
        Box::new(Id::new()),
        Box::new(Ping::new()),
        Box::new(Risc::new()),
        Box::new(Start::new()),
        Box::new(Test::new()),
    ];
}

/// The command handler.
pub struct Handler;

impl Handler {
    /// Handle the given command.
    pub fn handle(cmd: &str, msg: Message, api: &Api)
        -> Box<Future<Item = (), Error = Error>>
    {
        // Invoke the proper action
        let action = ACTIONS.iter()
            .find(|a| a.is_cmd(cmd));
        if let Some(action) = action {
            Box::new(
                action.invoke(&msg, api)
                    .map_err(|err| ActionError::Invoke(err.compat()))
                    .from_err()
            )
        } else {
            Box::new(ok(()))
        }
    }
}

/// A command handler error.
#[derive(Debug, Fail)]
pub enum Error {
    /// An error occurred while handling a command.
    #[fail(display = "failed to execute command")]
    Cmd(#[cause] ActionError),
}

/// Convert command action errors to a command handler error.
impl From<ActionError> for Error {
    fn from(err: ActionError) -> Error {
        Error::Cmd(err)
    }
}
