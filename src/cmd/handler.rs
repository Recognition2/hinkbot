use futures::{
    Future,
    future::ok,
};
use telegram_bot::{
    Api,
    types::Message,
};

use super::action::Action;
use super::action::exec::Exec;
use super::action::genimi::Genimi;
use super::action::help::Help;
use super::action::ping::Ping;
use super::action::test::Test;

lazy_static! {
    static ref ACTIONS: Vec<Box<dyn Action + Sync>> = vec![
        Box::new(Exec::new()),
        Box::new(Genimi::new()),
        Box::new(Help::new()),
        Box::new(Ping::new()),
        Box::new(Test::new()),
    ];
}

/// The command handler.
pub struct Handler;

impl Handler {
    /// Handle the given command.
    pub fn handle(cmd: &str, msg: Message, api: &Api)
        -> Box<Future<Item = (), Error = ()>>
    {
        // Invoke the proper action
        if let Some(action) = ACTIONS.iter()
            .find(|a| a.is_cmd(cmd))
        {
            action.invoke(&msg, api)
        } else {
            Box::new(ok(()))
        }
    }
}
