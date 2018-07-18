use std::collections::HashMap;
use std::sync::Mutex;

use diesel::{
    mysql::MysqlConnection,
    prelude::*,
    result::Error as DieselError,
    self,
};
use telegram_bot::types::{ChatId, MessageKind, UserId};

use models::{Chat, ChatUserStats, User};
use schema::{chat, chat_user_stats, user};

pub struct Stats {
    /// A queue of stats that still needs to be pushed to the database.
    queue: Mutex<HashMap<ChatId, HashMap<UserId, HashMap<StatsKind, (u32, u32)>>>>,
}

impl Stats {
    /// Constructor.
    pub fn new() -> Stats {
        Stats {
            queue: Mutex::new(HashMap::new()),
        }
    }

    /// Increase the total message and edits count for the given user in the given chat.
    /// The update is pushed to the queue, to be pushed to the database periodically.
    /// If the given message kind is not a counted stat, nothing happends.
    pub fn increase_stats(
        &self,
        chat: ChatId,
        user: UserId,
        kind: &MessageKind,
        messages: u32,
        edits: u32,
    ) {
        if let Some(message_type) = StatsKind::from_message_kind(kind) {
            match self.queue.lock() {
                Ok(ref mut queue) => {
                    let entry = queue.entry(chat)
                        .or_insert(HashMap::new())
                        .entry(user)
                        .or_insert(HashMap::new())
                        .entry(message_type)
                        .or_insert((0, 0));
                    entry.0 += messages;
                    entry.0 += edits;
                },
                Err(_) => eprintln!("ERR: failed lock stats queue, unable to increase user stats"),
            }
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
        chats: &mut HashMap<ChatId, HashMap<UserId, HashMap<StatsKind, (u32, u32)>>>,
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
        users: &mut HashMap<UserId, HashMap<StatsKind, (u32, u32)>>,
        connection: &MysqlConnection,
    ) {
        // Flush all users in this chat, remove successfully flushed
        users.retain(|user, stats| {
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

            // Flush the the user to the database, retain users that have stats that failed
            Self::flush_user(chat, *user, stats, connection);
            !stats.is_empty()
        });
    }

    /// Flush all users stats for all availalbe message types.
    /// Data for users successfully pushed to the database, is removed from the hashmap.
    /// Any errors while flushing are reported in the console.
    pub fn flush_user(
        chat: ChatId,
        user: UserId,
        stats: &mut HashMap<StatsKind, (u32, u32)>,
        connection: &MysqlConnection,
    ) {
        // Flush all message types for the given user, remove successfully flushed
        stats.retain(|message_type, (messages, edits)| {
            let result = Self::flush_user_stats(chat, user, *message_type, *messages, *edits, connection);
            if let Err(ref err) = result {
                eprintln!(
                    "ERR: failed to flush chat user stats to database, skipping: {}",
                    err,
                );
            }
            result.is_err()
        });
    }

    /// Flush the given user stats in a chat to the database.
    /// The user stats item is created if it doesn't exist yet.
    /// If the operation failed, an error is returned.
    pub fn flush_user_stats(
        chat: ChatId,
        user: UserId,
        message_type: StatsKind,
        messages: u32,
        edits: u32,
        connection: &MysqlConnection,
    ) -> Result<(), DieselError> {
        // Find an existing entry in the database and update it, or create a new entry
        match chat_user_stats::dsl::chat_user_stats
            .find((chat.to_i64(), user.to_i64(), message_type.to_id()))
            .first::<ChatUserStats>(connection)
        {
            Ok(existing) =>
                diesel::update(&existing)
                    .set((
                        chat_user_stats::dsl::message_type.eq(message_type.to_id()),
                        chat_user_stats::dsl::messages.eq(chat_user_stats::dsl::messages + messages as i32),
                        chat_user_stats::dsl::edits.eq(chat_user_stats::dsl::edits + edits as i32),
                    ))
                    .execute(connection)
                    .map(|_| ()),
            Err(DieselError::NotFound) =>
                diesel::insert_into(chat_user_stats::dsl::chat_user_stats)
                    .values((
                        chat_user_stats::dsl::chat_id.eq(chat.to_i64()),
                        chat_user_stats::dsl::user_id.eq(user.to_i64()),
                        chat_user_stats::dsl::message_type.eq(message_type.to_id()),
                        chat_user_stats::dsl::messages.eq(messages as i32),
                        chat_user_stats::dsl::edits.eq(edits as i32),
                    ))
                    .execute(connection)
                    .map(|_| ()),
            err => err.map(|_| ()),
        }
    }
}

/// Types of stats.
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub enum StatsKind {
    Text,
    Audio,
    Document,
    Photo,
    Sticker,
    Video,
    Voice,
    VideoNote,
    Contact,
    Location,
    Venue,
    ChatTitle,
    ChatPhoto,
    PinnedMessage,
}

impl StatsKind {
    /// Get the stats kind for the given message kind.
    /// Some kinds do not have a corresponding stats kind, `None` will be returned for these.
    fn from_message_kind(kind: &MessageKind) -> Option<Self> {
        match kind {
            MessageKind::Text { .. } => Some(StatsKind::Text),
            MessageKind::Audio { .. } => Some(StatsKind::Audio),
            MessageKind::Document { .. } => Some(StatsKind::Document),
            MessageKind::Photo { .. } => Some(StatsKind::Photo),
            MessageKind::Sticker { .. } => Some(StatsKind::Sticker),
            MessageKind::Video { .. } => Some(StatsKind::Video),
            MessageKind::Voice { .. } => Some(StatsKind::Voice),
            MessageKind::VideoNote { .. } => Some(StatsKind::VideoNote),
            MessageKind::Contact { .. } => Some(StatsKind::Contact),
            MessageKind::Location { .. } => Some(StatsKind::Location),
            MessageKind::Venue { .. } => Some(StatsKind::Venue),
            MessageKind::NewChatMembers { .. } => None,
            MessageKind::LeftChatMember { .. } => None,
            MessageKind::NewChatTitle { .. } => Some(StatsKind::ChatTitle),
            MessageKind::NewChatPhoto { .. } => Some(StatsKind::ChatPhoto),
            MessageKind::DeleteChatPhoto => Some(StatsKind::ChatPhoto),
            MessageKind::GroupChatCreated => None,
            MessageKind::SupergroupChatCreated => None,
            MessageKind::ChannelChatCreated => None,
            MessageKind::MigrateToChatId { .. } => None,
            MessageKind::MigrateFromChatId { .. } => None,
            MessageKind::PinnedMessage { .. } => Some(StatsKind::PinnedMessage),
            MessageKind::Unknown { .. } => None,
        }
    }

    /// Get the stats kind for the given ID.
    /// If the given ID is invalid, `None` is returned.
    pub fn from_id(&self, id: i16) -> Option<StatsKind> {
        match id {
            1 => Some(StatsKind::Text),
            2 => Some(StatsKind::Audio),
            3 => Some(StatsKind::Document),
            4 => Some(StatsKind::Photo),
            5 => Some(StatsKind::Sticker),
            6 => Some(StatsKind::Video),
            7 => Some(StatsKind::Voice),
            8 => Some(StatsKind::VideoNote),
            9 => Some(StatsKind::Contact),
            10 => Some(StatsKind::Location),
            11 => Some(StatsKind::Venue),
            12 => Some(StatsKind::ChatTitle),
            13 => Some(StatsKind::ChatPhoto),
            14 => Some(StatsKind::PinnedMessage),
            _ => None,
        }
    }

    /// Get the corresponding ID for the stats kind.
    pub fn to_id(&self) -> i16 {
        match self {
            StatsKind::Text => 1,
            StatsKind::Audio => 2,
            StatsKind::Document => 3,
            StatsKind::Photo => 4,
            StatsKind::Sticker => 5,
            StatsKind::Video => 6,
            StatsKind::Voice => 7,
            StatsKind::VideoNote => 8,
            StatsKind::Contact => 9,
            StatsKind::Location => 10,
            StatsKind::Venue => 11,
            StatsKind::ChatTitle => 12,
            StatsKind::ChatPhoto => 13,
            StatsKind::PinnedMessage => 14,
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
