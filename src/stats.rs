use std::collections::HashMap;
use std::sync::Mutex;

use chrono::NaiveDateTime;
use diesel::{
    mysql::MysqlConnection,
    prelude::*,
    result::{
        Error as DieselError,
        QueryResult,
    },
    self,
};
use telegram_bot::types::{ChatId, Message, MessageKind, UserId};

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
        message: &Message,
        messages: u32,
        edits: u32,
    ) {
        if let Some(message_type) = StatsKind::from_message(message) {
            match self.queue.lock() {
                Ok(ref mut queue) => {
                    let entry = queue.entry(message.chat.id())
                        .or_insert(HashMap::new())
                        .entry(message.from.id)
                        .or_insert(HashMap::new())
                        .entry(message_type)
                        .or_insert((0, 0));
                    entry.0 += messages;
                    entry.1 += edits;
                },
                Err(_) => eprintln!("ERR: failed lock stats queue, unable to increase user stats"),
            }
        }
    }

    /// Increase the total message and edits count for the given message.
    /// The update is pushed to the queue, to be pushed to the database periodically.
    /// If the given message kind is not a counted stat, nothing happends.
    pub fn increase_message_stats(&self, message: &Message, messages: u32, edits: u32) {
        self.increase_stats(message, messages, edits);
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
            .find((chat.to_i64(), user.to_i64(), message_type.id()))
            .first::<ChatUserStats>(connection)
        {
            Ok(existing) =>
                diesel::update(&existing)
                    .set((
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
                        chat_user_stats::dsl::message_type.eq(message_type.id()),
                        chat_user_stats::dsl::messages.eq(messages as i32),
                        chat_user_stats::dsl::edits.eq(edits as i32),
                    ))
                    .execute(connection)
                    .map(|_| ()),
            err => err.map(|_| ()),
        }
    }

    /// Fetch chat stats.
    pub fn fetch_chat_stats(&self, connection: &MysqlConnection, chat: ChatId)
        -> QueryResult<ChatStats>
    {
        use self::chat_user_stats::dsl::{
            chat_id,
            chat_user_stats,
            created_at,
            edits,
            messages,
            user_id,
        };
        //use self::diesel::dsl::sum;

        // Get all message stats associated with this chat
        let all_stats: Vec<(i64, i32, i32)> = chat_user_stats
            .select((user_id, messages, edits))
            .filter(chat_id.eq(chat.to_i64()))
            .load(connection)?;

        // Build a list of user totals, grouped by the user
        let mut user_totals: HashMap<i64, (i32, i32)> = HashMap::new();
        for (user, num_messages, num_edits) in all_stats {
            let entry = user_totals.entry(user).or_insert((0, 0));
            entry.0 += num_messages;
            entry.1 += num_edits;
        }
        let mut user_totals: Vec<(String, i64, i32, i32)> = user_totals
            .into_iter()
            .map(|(user, (num_messages, num_edits))|
                 (format!("{}", user), user, num_messages, num_edits)
            )
            .collect();
        user_totals.sort_unstable_by(|a, b| (b.2 + b.3).cmp(&(a.2 + a.3)));

        // Get message totals for this chat
        let total_messages = user_totals.iter().map(|(_, _, n, _)| n).sum();
        let total_edits = user_totals.iter().map(|(_, _, _, n)| n).sum();

        // Get the time we started recording stats at
        let since = chat_user_stats
            .select(created_at)
            .filter(chat_id.eq(chat.to_i64()))
            .order(created_at.asc())
            .first::<NaiveDateTime>(connection)
            .ok();

        // Build the chat stats
        Ok(ChatStats::new(user_totals, total_messages, total_edits, since))
    }
}

pub struct ChatStats {
    /// A list of users and the number of messages and edits they made.
    /// This vector is sorted from largest to lowest number of edits.
    /// The following format is used: `(user name, user ID, messages, edits)`.
    users: Vec<(String, i64, i32, i32)>,

    /// The total number of messages.
    total_messages: i32,

    /// The total number of edits.
    total_edits: i32,

    /// The time since these stats were recorded.
    since: Option<NaiveDateTime>,
}

impl ChatStats {
    /// Constructor.
    pub fn new(
        users: Vec<(String, i64, i32, i32)>,
        total_messages: i32,
        total_edits: i32,
        since: Option<NaiveDateTime>,
    ) -> Self {
        ChatStats {
            users,
            total_messages,
            total_edits,
            since,
        }
    }

    /// Get the user totals.
    pub fn users(&self) -> &Vec<(String, i64, i32, i32)> {
        &self.users
    }

    /// Get the total number of messages
    pub fn total_messages(&self) -> i32 {
        self.total_messages
    }

    /// Get the total number of edits
    pub fn total_edits(&self) -> i32 {
        self.total_edits
    }

    /// Get the time since message stats were recorded.
    pub fn since(&self) -> &Option<NaiveDateTime> {
        &self.since
    }
}

/// Types of stats.
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub enum StatsKind {
    Text,
    Command,
    Audio,
    Document,
    Gif,
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
    Forward,
}

impl StatsKind {
    /// Get the stats kind for the given message kind.
    /// Some kinds do not have a corresponding stats kind, `None` will be returned for these.
    fn from_message(message: &Message) -> Option<Self> {
        // Check whether this message was forwarded
        if message.forward.is_some() {
            return Some(StatsKind::Forward);
        }

        // Determine the stats kind based on the message kind
        match &message.kind {
            MessageKind::Text { data, .. } => if data.trim_left().starts_with('/') {
                    Some(StatsKind::Command)
                } else {
                    Some(StatsKind::Text)
                },
            MessageKind::Audio { .. } => Some(StatsKind::Audio),
            MessageKind::Document { data, .. } => {
                    // If the MIME type is a gif, it must be a GIF
                    if data.mime_type == Some("image/gif".into()) {
                        return Some(StatsKind::Gif);
                    }

                    // If the mime type is MP4, and the filename is from Giphy, it may be a GIF
                    if data.mime_type == Some("video/mp4".into())
                        && data.file_name == Some("giphy.mp4".into())
                    {
                        return Some(StatsKind::Gif);
                    }

                    Some(StatsKind::Document)
                },
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
    pub fn _from_id(&self, id: i16) -> Option<StatsKind> {
        match id {
            1 => Some(StatsKind::Text),
            2 => Some(StatsKind::Command),
            3 => Some(StatsKind::Audio),
            4 => Some(StatsKind::Document),
            5 => Some(StatsKind::Gif),
            6 => Some(StatsKind::Photo),
            7 => Some(StatsKind::Sticker),
            8 => Some(StatsKind::Video),
            9 => Some(StatsKind::Voice),
            10 => Some(StatsKind::VideoNote),
            11 => Some(StatsKind::Contact),
            12 => Some(StatsKind::Location),
            13 => Some(StatsKind::Venue),
            14 => Some(StatsKind::ChatTitle),
            15 => Some(StatsKind::ChatPhoto),
            16 => Some(StatsKind::PinnedMessage),
            17 => Some(StatsKind::Forward),
            _ => None,
        }
    }

    /// Get the corresponding ID for the stats kind.
    pub fn id(&self) -> i16 {
        match self {
            StatsKind::Text => 1,
            StatsKind::Command => 2,
            StatsKind::Audio => 3,
            StatsKind::Document => 4,
            StatsKind::Gif => 5,
            StatsKind::Photo => 6,
            StatsKind::Sticker => 7,
            StatsKind::Video => 8,
            StatsKind::Voice => 9,
            StatsKind::VideoNote => 10,
            StatsKind::Contact => 11,
            StatsKind::Location => 12,
            StatsKind::Venue => 13,
            StatsKind::ChatTitle => 14,
            StatsKind::ChatPhoto => 15,
            StatsKind::PinnedMessage => 16,
            StatsKind::Forward => 17,
        }
    }

    /// Get the name for the current stats kind.
    pub fn name(&self) -> &'static str {
        match self {
            StatsKind::Text => "text message",
            StatsKind::Command => "command",
            StatsKind::Audio => "audio message",
            StatsKind::Document => "document",
            StatsKind::Gif => "GIF",
            StatsKind::Photo => "photo",
            StatsKind::Sticker => "sticker",
            StatsKind::Video => "video",
            StatsKind::Voice => "voice message",
            StatsKind::VideoNote => "video note",
            StatsKind::Contact => "contact",
            StatsKind::Location => "location",
            StatsKind::Venue => "venue",
            StatsKind::ChatTitle => "changed chat title",
            StatsKind::ChatPhoto => "changed chat photo",
            StatsKind::PinnedMessage => "pinned",
            StatsKind::Forward => "forwarded",
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
