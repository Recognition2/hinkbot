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
    queue: Mutex<HashMap<ChatId, HashMap<UserId, i32>>>,
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

    /// Flush the queue with stats to the database.
    // TODO: greatly improve the logic in here, it's a mess currently
    pub fn flush(&self, connection: &MysqlConnection) {
        match self.queue.lock() {
            Ok(ref mut queue) => {
                // TODO: set the proper title here
                // TODO: set the update time

                // Loop through all the chats in the queu
                for (new_chat, users) in queue.iter_mut() {
                    // Ensure the chat exists in the database
                    match chat::dsl::chat.find(new_chat.to_i64()).first::<Chat>(connection) {
                        Ok(mut _db_chat) => {
                            // TODO: update if the title changed, continue on error
                            // diesel::update(&db_chat)
                            //     .set((title.eq("title")))
                            //     .execute(connection);
                        },
                        Err(DieselError::NotFound) =>
                            if let Err(err) = diesel::insert_into(chat::dsl::chat)
                                .values(chat::dsl::telegram_id.eq(new_chat.to_i64()))
                                .execute(connection)
                            {
                                println!("ERR: failed to create queued chat in database, skipping: {}", err);
                                continue;
                            },
                        Err(_) => {
                            println!("ERR: failed to check if queued chat exists in the database, skipping");
                            continue;
                        },
                    }

                    // A vector holding the IDs for users the stats were successfully updated for
                    let mut successful_users = vec![];

                    // Loop through the users in this chat
                    for (new_user, messages) in users.iter_mut() {
                        // Ensure the user exists in the database
                        match user::dsl::user.find(new_user.to_i64()).first::<User>(connection) {
                            Ok(mut _db_user) => {
                                // TODO: update if the title changed, continue on error
                                // diesel::update(&db_user)
                                //     .set((first_name.eq("First name"), last_name.eq("Last name")))
                                //     .execute(connection);
                            },
                            Err(DieselError::NotFound) =>
                                if let Err(err) = diesel::insert_into(user::dsl::user)
                                    .values(user::dsl::telegram_id.eq(new_user.to_i64()))
                                    .execute(connection)
                                {
                                    println!("ERR: failed to create queued user in database, skipping: {}", err);
                                    continue;
                                },
                            Err(_) => {
                                println!("ERR: failed to check if queued user exists in the database, skipping");
                                continue;
                            },
                        }

                        // Find and update or create the corresponding chat user
                        let db_chat_user = chat_user_stats::dsl::chat_user_stats
                            .find((new_chat.to_i64(), new_user.to_i64()))
                            .first::<ChatUserStats>(connection);
                        match db_chat_user {
                            Ok(mut db_chat_user) =>
                                match diesel::update(&db_chat_user)
                                    .set(chat_user_stats::dsl::messages.eq(chat_user_stats::dsl::messages + *messages))
                                    .execute(connection)
                                {
                                    Ok(_) => successful_users.push(new_user),
                                    Err(err) => {
                                        println!("ERR: failed to create user chat stats in database, skipping: {}", err);
                                        continue;
                                    },
                                },
                            Err(DieselError::NotFound) =>
                                match diesel::insert_into(chat_user_stats::dsl::chat_user_stats)
                                    .values((
                                        chat_user_stats::dsl::chat_id.eq(new_chat.to_i64()),
                                        chat_user_stats::dsl::user_id.eq(new_user.to_i64()),
                                        chat_user_stats::dsl::messages.eq(*messages),
                                    ))
                                    .execute(connection)
                                {
                                    Ok(_) => successful_users.push(new_user),
                                    Err(err) => {
                                        println!("ERR: failed to update user chat stats in database, skipping: {}", err);
                                        continue;
                                    },
                                },
                            Err(_) => {
                                println!("ERR: failed to check if queued chat user stats exists in the database, skipping");
                                continue;
                            },
                        }
                    }

                    // // Remove user stats that have successfully been pushed to the database
                    // for user in successful_users {
                    //     users.remove(user);
                    // }
                }

                // // Remove chats that don't have any users anymore
                // let remove_chats: Vec<&ChatId> = queue.iter()
                //     .filter(|(_, users)| users.is_empty())
                //     .map(|(chat, _)| chat)
                //     .collect();
                // for chat in remove_chats {
                //     queue.remove(chat);
                // }

                // TODO: only clear queue items that were successfully pushed to the database
                queue.clear();
            },
            Err(_) => println!("ERR: failed lock stats queue, unable to flush to database"),
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
