use std::collections::HashMap;
use std::sync::Mutex;

use telegram_bot::types::{ChatId, UserId};

pub struct Stats {
    /// A queue of stats that still needs to be pushed to the database.
    queue: Mutex<HashMap<ChatId, HashMap<UserId, u32>>>,
}

impl Stats {
    /// Constructor.
    pub fn new() -> Stats {
        Stats {
            queue: Mutex::new(HashMap::new()),
        }
    }

    /// Increase the total message count for the given user in the given chat.
    /// The update is pushed to the queue, to be pushed to the database periodically.
    pub fn increase_messages(&self, chat: ChatId, user: UserId) {
        match self.queue.lock() {
            Ok(ref mut queue) =>
                *queue.entry(chat)
                    .or_insert(HashMap::new())
                    .entry(user)
                    .or_insert(0) += 1,
            Err(_) => println!("ERR: failed lock stats queue, unable to increase user stats"),
        }
    }
}
