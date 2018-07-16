use std::collections::HashMap;
use std::sync::Mutex;

use diesel::{
    mysql::MysqlConnection,
    prelude::*,
    result::Error as DieselError,
    self,
};
use telegram_bot::types::{ChatId, UserId};

use models::{Chat, ChatUserStats, User};
use schema::{chat, chat_user_stats, user};

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
            Err(_) => eprintln!("ERR: failed lock stats queue, unable to increase user stats"),
        }
    }

    /// Flush the queue with stats to the database.
    /// Items successfully pushed to the database are cleared from the queue.
    /// Any errors while flushing are reported in the console.
    pub fn flush(&self, connection: &MysqlConnection) {
        match self.queue.lock() {
            Ok(ref mut chats) => Self::flush_chats(chats, connection),
            Err(err) => eprintln!("ERR: failed lock stats queue, unable to flush to database: {}", err),
        }
    }

    /// Flush all chats from the queue to the database
    /// Items not successfully flashed are retained in the given list, other items are removed.
    /// Any errors while flushing are reported in the console.
    pub fn flush_chats(
        chats: &mut HashMap<ChatId, HashMap<UserId, u32>>,
        connection: &MysqlConnection,
    ) {
        // Flush each chat, remove the successfully flushed
        chats.retain(|chat, ref mut users| {
            // Find an existing entry in the database and update it, or create a new entry
            match chat::dsl::chat.find(chat.to_i64()).first::<Chat>(connection) {
                Ok(mut _existing) => {
                    // TODO: update if the title changed
                    // diesel::update(&existing)
                    //     .set((title.eq("title")))
                    //     .execute(connection);
                },
                Err(DieselError::NotFound) =>
                    if let Err(err) = diesel::insert_into(chat::dsl::chat)
                        .values(chat::dsl::telegram_id.eq(chat.to_i64()))
                        .execute(connection)
                    {
                        eprintln!("ERR: failed to create queued chat in database, skipping: {}", err);
                        return false;
                    },
                Err(err) => {
                    eprintln!("ERR: failed to check if queued chat exists in the database, skipping: {}", err);
                    return false;
                },
            }

            // Flush all users for this chat to the database
            Self::flush_users(*chat, users, connection);

            // Remove the chat entry if the users list is now empty
            !users.is_empty()
        });
    }

    /// Flush all users from the given hashmap for a chat to the database.
    /// If a user doesn't have a record in the database yet, it is created.
    /// Data for users successfully pushed to the database, is removed from the hashmap.
    /// Any errors while flushing are reported in the console.
    pub fn flush_users(
        chat: ChatId,
        users: &mut HashMap<UserId, u32>,
        connection: &MysqlConnection,
    ) {
        // Flush all users in this chat, remove successfully flushed
        users.retain(|user, messages| {
            // Find an existing entry in the database and update it, or create a new entry
            match user::dsl::user.find(user.to_i64()).first::<User>(connection) {
                Ok(mut _existing) => {
                    // TODO: update if the name changed
                    // diesel::update(&existing)
                    //     .set((first_name.eq("First name"), last_name.eq("Last name")))
                    //     .execute(connection);
                },
                Err(DieselError::NotFound) =>
                    if let Err(err) = diesel::insert_into(user::dsl::user)
                        .values(user::dsl::telegram_id.eq(user.to_i64()))
                        .execute(connection)
                    {
                        eprintln!("ERR: failed to create queued user in database, skipping: {}", err);
                        return true;
                    },
                Err(err) => {
                    eprintln!("ERR: failed to check if queued user exists in the database, skipping: {}", err);
                    return true;
                },
            }

            // Flush the user stats to the database, keep them in the list on error
            let result = Self::flush_user(chat, *user, *messages, connection);
            if let Err(ref err) = result {
                eprintln!("ERR: failed to flush chat user stats to database, skipping: {}", err);
            }
            result.is_err()
        });
    }

    /// Flush the given user stats in a chat to the database.
    /// The user stats item is created if it doesn't exist yet.
    /// If the operation failed, an error is returned.
    pub fn flush_user(
        chat: ChatId,
        user: UserId,
        messages: u32,
        connection: &MysqlConnection,
    ) -> Result<(), DieselError> {
        // Find an existing entry in the database and update it, or create a new entry
        match chat_user_stats::dsl::chat_user_stats
            .find((chat.to_i64(), user.to_i64()))
            .first::<ChatUserStats>(connection)
        {
            Ok(existing) =>
                diesel::update(&existing)
                    .set(chat_user_stats::dsl::messages.eq(chat_user_stats::dsl::messages + messages as i32))
                    .execute(connection)
                    .map(|_| ()),
            Err(DieselError::NotFound) =>
                diesel::insert_into(chat_user_stats::dsl::chat_user_stats)
                    .values((
                        chat_user_stats::dsl::chat_id.eq(chat.to_i64()),
                        chat_user_stats::dsl::user_id.eq(user.to_i64()),
                        chat_user_stats::dsl::messages.eq(messages as i32),
                    ))
                    .execute(connection)
                    .map(|_| ()),
            err => err.map(|_| ()),
        }
    }
}

// TODO: find something better for this
trait TelegramToI64 {
    fn to_i64(&self) -> i64;
}

impl TelegramToI64 for ChatId {
    fn to_i64(&self) -> i64 {
        self.to_string().parse().unwrap()
    }
}

impl TelegramToI64 for UserId {
    fn to_i64(&self) -> i64 {
        self.to_string().parse().unwrap()
    }
}
