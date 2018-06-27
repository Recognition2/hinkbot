pub mod genimi;
pub mod ping;
pub mod test;

use futures::Future;
use telegram_bot::{
    Api,
    types::Message,
};

pub trait Action {
    /// Get the command name for this action.
    ///
    /// The name of the command `/ping` would be `ping`.
    /// The returned value should be in lowercase and must not contain any whitespace.
    fn cmd(&self) -> &'static str;

    /// Check whether this action is for the command with the given name.
    ///
    /// The name of the command `/ping` would be `ping`.
    fn is_cmd(&self, cmd: &str) -> bool {
        cmd.trim().eq_ignore_ascii_case(self.cmd())
    }

    /// Invoke the action with the given context.
    fn invoke(&self, msg: &Message, api: &Api) -> Box<Future<Item = (), Error = ()>>;
}
