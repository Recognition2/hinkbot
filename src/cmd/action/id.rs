use std::time::Duration;

use chrono::{
    DateTime,
    NaiveDateTime,
    Utc,
};
use futures::{
    future::ok,
    Future,
};
use humansize::{FileSize, file_size_opts};
use humantime::format_duration;
use telegram_bot::{
    Api,
    prelude::*,
    types::{
        ChannelPost,
        ForwardFrom,
        MessageChat,
        Message,
        MessageKind,
        MessageOrChannelPost,
        ParseMode,
        User,
    },
};

use super::Action;

/// The action command name.
const CMD: &'static str = "id";

/// The action help.
const HELP: &'static str = "Show hidden message/chat details";

pub struct Id;

impl Id {
    pub fn new() -> Self {
        Id
    }

    /// Build the info part for the given user.
    pub fn build_user_info(user: &User, caption: &str) -> String {
        // Build the header, add the ID
        let mut info = format!("\
                *{}:*\n\
                ID: _{}_\
            ",
            caption,
            user.id,
        );

        // Apend the username if known
        if let Some(ref username) = user.username {
            info += &format!(
                "\nUsername: _{}_",
                username,
            );
        }

        // Append the name
        if let Some(ref last_name) = user.last_name {
            info += &format!("\n\
                    First name: _{}_\n\
                    Last name: _{}_\
                ",
                user.first_name,
                last_name,
            );
        } else {
            info += &format!(
                "\nName: _{}_",
                user.first_name,
            );
        }

        info
    }

    /// Build the info part for the given chat message.
    // TODO: the `build_channel_post_info` message is similar, merge them
    pub fn build_msg_info(msg: &Message, caption: &str) -> String {
        // Build the main info
        let mut info = format!("\
                *{}:*\n\
                ID: _{}_\n\
                Poster ID: _{}_\n\
                Chat ID: _{}_\n\
                {}\n\
                Date: _{}_\
            ",
            caption,
            msg.id,
            msg.from.id,
            msg.chat.id(),
            Self::build_message_kind_details(&msg.kind),
            Self::format_timestamp(msg.date),
        );

        // Append the edit date if edited
        if let Some(date) = msg.edit_date {
            info += &format!(
                "\nEdit date: _{}_",
                Self::format_timestamp(date),
            );
        }

        // Append the reply details
        info += &format!(
            "\nIs reply: _{}_",
            Self::format_yes_no(msg.reply_to_message.is_some()),
        );

        // Append forwarder information if availalbe
        if let Some(ref forward) = msg.forward {
            info += &format!("\n\
                    Forwarded: _yes_\n\
                    Original date: _{}_\
                ",
                Self::format_timestamp(forward.date),
            );

            // Add source information
            info += &match &forward.from {
                ForwardFrom::User {
                        user,
                    } => format!(
                        "\nOriginal user ID: _{}_",
                        user.id,
                    ),
                ForwardFrom::Channel {
                        channel,
                        message_id,
                    } => format!("\n\
                            Original message ID: _{}_\n\
                            Original channel ID: _{}_\
                        ",
                        message_id,
                        channel.id,
                    ),
            };
        }

        info
    }

    /// Build the info part for the given channel post.
    pub fn build_channel_post_info(msg: &ChannelPost, caption: &str) -> String {
        // Build the main info
        let mut info = format!("\
                *{}:*\n\
                ID: _{}_\n\
                Channel ID: _{}_\n\
                Channel title: _{}_\n\
                {}\n\
                Date: _{}_\
            ",
            caption,
            msg.id,
            msg.chat.id,
            msg.chat.title,
            Self::build_message_kind_details(&msg.kind),
            Self::format_timestamp(msg.date),
        );

        // Append the edit date if edited
        if let Some(date) = msg.edit_date {
            info += &format!(
                "\nEdit date: _{}_",
                Self::format_timestamp(date),
            );
        }

        // Append the reply details
        info += &format!(
            "\nIs reply: _{}_",
            Self::format_yes_no(msg.reply_to_message.is_some()),
        );

        // Append forwarder information if availalbe
        if let Some(ref forward) = msg.forward {
            info += &format!("\n\
                    Forwarded: _yes_\n\
                    Original date: _{}_\
                ",
                Self::format_timestamp(forward.date),
            );

            // Add source information
            info += &match &forward.from {
                ForwardFrom::User {
                        user,
                    } => format!(
                        "\nOriginal user ID: _{}_",
                        user.id,
                    ),
                ForwardFrom::Channel {
                        channel,
                        message_id,
                    } => format!("\n\
                            Original message ID: _{}_\n\
                            Original channel ID: _{}_\
                        ",
                        message_id,
                        channel.id,
                    ),
            };
        }

        info
    }

    /// Build the info part for the given chat message or channel post.
    pub fn build_msg_channel_post_info(msg: &MessageOrChannelPost, caption: &str) -> String {
        match msg {
            MessageOrChannelPost::Message(msg) =>
                Self::build_msg_info(msg, caption),
            MessageOrChannelPost::ChannelPost(post) =>
                Self::build_channel_post_info(post, caption),
        }
    }

    /// Build the info part for the given chat.
    pub fn build_chat_info(chat: &MessageChat, caption: &str) -> String {
        // Build the base info
        let mut info = format!(
            "*{}:*",
            caption,
        );

        // Add details
        info += &match chat {
            MessageChat::Private(user) => format!("\n\
                    Type: _private_\n\
                    With user ID: _{}_\
                ",
                user.id,
            ),
            MessageChat::Group(group) => format!("\n\
                    Type: _group_\n\
                    Group ID: _{}_\n\
                    Title: _{}_\
                ",
                group.id,
                group.title,
            ),
            MessageChat::Supergroup(group) => format!("\n\
                    Type: _supergroup_\n\
                    Group ID: _{}_\n\
                    Title: _{}_\
                ",
                group.id,
                group.title,
            ),
            MessageChat::Unknown(_) =>
                "Type: _?_".into(),
        };

        info
    }

    /// Get `yes` or `no` if `true` or `false`.
    pub fn format_yes_no(b: bool) -> &'static str {
        if b {
            "yes"
        } else {
            "no"
        }
    }

    /// Format the given file size in a human readable format.
    pub fn format_file_size(size: &i64) -> String {
        size.file_size(file_size_opts::BINARY)
            .unwrap_or(format!("{} B", size))
    }

    /// Build message kind details.
    pub fn build_message_kind_details(kind: &MessageKind) -> String {
        match kind {
            MessageKind::Text { .. } => 
                String::from("Kind: _text_"),
            MessageKind::Audio {
                data,
            } => {
                // Build generic info
                let mut info = format!("\
                        Kind: _audio_\n\
                        Audio file ID: _{}_\n\
                        Audio length: _{}_\
                    ",
                    data.file_id,
                    format_duration(Duration::from_secs(data.duration as u64)),
                );

                // Append the performer name
                if let Some(ref performer) = data.performer {
                    info += &format!(
                        "\nAudio performer: _{}_",
                        performer,
                    );
                }

                // Append the title
                if let Some(ref title) = data.title {
                    info += &format!(
                        "\nAudio title: _{}_",
                        title,
                    );
                }

                // Append the mime type
                if let Some(ref mime) = data.mime_type {
                    info += &format!(
                        "\nAudio mime: _{}_",
                        mime,
                    );
                }

                // Append the file size
                if let Some(ref size) = data.file_size {
                    info += &format!(
                        "\nAudio size: _{} b_",
                        Self::format_file_size(size),
                    );
                }

                info
            },
            MessageKind::Document {
                data,
                caption,
            } => {
                // Build generic info
                let mut info = format!("\
                        Kind: _document_\n\
                        Document file ID: _{}_\
                    ",
                    data.file_id,
                );

                // Append the thumbnail information
                if let Some(ref thumb) = data.thumb {
                    info += &format!("\n\
                            Document thumb file ID: _{}_\n\
                            Document pixels: _{}x{}_\
                        ",
                        thumb.file_id,
                        thumb.width,
                        thumb.height,
                    );

                    // Append the thumbnail file size
                    if let Some(ref size) = thumb.file_size {
                        info += &format!(
                            "\nDocument size: _{}_",
                            Self::format_file_size(size),
                        );
                    }
                }

                // Append the file name
                if let Some(ref name) = data.file_name {
                    info += &format!(
                        "\nDocument name: _{}_",
                        name,
                    );
                }

                // Append the mime type
                if let Some(ref mime) = data.mime_type {
                    info += &format!(
                        "\nAudio mime: _{}_",
                        mime,
                    );
                }

                // Append the file size
                if let Some(ref size) = data.file_size {
                    info += &format!(
                        "\nDocument size: _{}_",
                        Self::format_file_size(size),
                    );
                }

                // Append the caption
                if let Some(caption) = caption {
                    info += &format!(
                        "\nDocument caption: _{}_",
                        caption,
                    );
                }

                info
            },
            MessageKind::Photo {
                data,
                caption,
            } => {
                // Build generic info
                let mut info = format!("\
                        Kind: _photo_\n\
                        Photos: _{}_\
                    ",
                    data.len(),
                );

                // Loop through the photos
                for (i, photo) in data.iter().enumerate() {
                    info += &format!("\n\
                            Photo #{} file ID: _{}_\n\
                            Photo #{} pixels: _{}x{}_\
                        ",
                        i,
                        photo.file_id,
                        i,
                        photo.width,
                        photo.height,
                    );

                    // Append the thumbnail file size
                    if let Some(ref size) = photo.file_size {
                        info += &format!(
                            "\nPhoto #{} size: _{}_",
                            i,
                            Self::format_file_size(size),
                        );
                    }
                }

                // Append the caption
                if let Some(caption) = caption {
                    info += &format!(
                        "\nDocument caption: _{}_",
                        caption,
                    );
                }

                info
            },
            MessageKind::Sticker {
                data,
            } => {
                // Build generic info
                let mut info = format!("\
                        Kind: _sticker_\n\
                        Sticker file ID: _{}_\n\
                        Sticker pixels: _{}x{}_\
                    ",
                    data.file_id,
                    data.width,
                    data.height,
                );

                // Append the thumbnail information
                if let Some(ref thumb) = data.thumb {
                    info += &format!("\n\
                            Sticker thumb file ID: _{}_\n\
                            Sticker thumb pixels: _{}x{}_\
                        ",
                        thumb.file_id,
                        thumb.width,
                        thumb.height,
                    );

                    // Append the thumbnail file size
                    if let Some(ref size) = thumb.file_size {
                        info += &format!(
                            "\nSticker thumb size: _{}_",
                            Self::format_file_size(size),
                        );
                    }
                }

                // Append the emojis
                if let Some(ref emoji) = data.emoji {
                    info += &format!(
                        "\nSticker emoji: _{}_",
                        emoji,
                    );
                }

                // Append the file size
                if let Some(ref size) = data.file_size {
                    info += &format!(
                        "\nSticker size: _{}_",
                        Self::format_file_size(size),
                    );
                }

                info
            },
            MessageKind::Video {
                data,
                caption,
            } => {
                // Build generic info
                let mut info = format!("\
                        Kind: _video_\n\
                        Video file ID: _{}_\n\
                        Video pixels: _{}x{}_\n\
                        Video length: _{}_\
                    ",
                    data.file_id,
                    data.width,
                    data.height,
                    format_duration(Duration::from_secs(data.duration as u64)),
                );

                // Append the thumbnail information
                if let Some(ref thumb) = data.thumb {
                    info += &format!("\n\
                            Video thumb file ID: _{}_\n\
                            Video thumb pixels: _{}x{}_\
                        ",
                        thumb.file_id,
                        thumb.width,
                        thumb.height,
                    );

                    // Append the thumbnail file size
                    if let Some(ref size) = thumb.file_size {
                        info += &format!(
                            "\nVideo thumb size: _{}_",
                            Self::format_file_size(size),
                        );
                    }
                }

                // Append the mime type
                if let Some(ref mime) = data.mime_type {
                    info += &format!(
                        "\nVideo file mime: _{}_",
                        mime,
                    );
                }

                // Append the file size
                if let Some(ref size) = data.file_size {
                    info += &format!(
                        "\nVideo size: _{}_",
                        Self::format_file_size(size),
                    );
                }

                // Append the caption
                if let Some(caption) = caption {
                    info += &format!(
                        "\nVideo caption: _{}_",
                        caption,
                    );
                }

                info
            },
            MessageKind::Voice {
                data,
            } => {
                // Build generic info
                let mut info = format!("\
                        Kind: _voice_\n\
                        Voice file ID: _{}_\n\
                        Voice length: _{}_\
                    ",
                    data.file_id,
                    format_duration(Duration::from_secs(data.duration as u64)),
                );

                // Append the mime type
                if let Some(ref mime) = data.mime_type {
                    info += &format!(
                        "\nVoice file mime: _{}_",
                        mime,
                    );
                }

                // Append the file size
                if let Some(ref size) = data.file_size {
                    info += &format!(
                        "\nVoice size: _{}_",
                        Self::format_file_size(size),
                    );
                }

                info
            },
            MessageKind::VideoNote {
                data,
            } => {
                // Build generic info
                let mut info = format!("\
                        Kind: _video note_\n\
                        Note file ID: _{}_\n\
                        Note length: _{}_\
                    ",
                    data.file_id,
                    format_duration(Duration::from_secs(data.duration as u64)),
                );

                // Append the thumbnail information
                if let Some(ref thumb) = data.thumb {
                    info += &format!("\n\
                            Note thumb file ID: _{}_\n\
                            Note thumb pixels: _{}x{}_\
                        ",
                        thumb.file_id,
                        thumb.width,
                        thumb.height,
                    );

                    // Append the thumbnail file size
                    if let Some(ref size) = thumb.file_size {
                        info += &format!(
                            "\nNote thumb size: _{}_",
                            Self::format_file_size(size),
                        );
                    }
                }

                // Append the file size
                if let Some(ref size) = data.file_size {
                    info += &format!(
                        "\nNote size: _{}_",
                        Self::format_file_size(size),
                    );
                }

                info
            },
            MessageKind::Contact {
                data,
            } => {
                // Build generic info
                let mut info = format!("\
                        Kind: _contact_\n\
                        Contact phone: _{}_\n\
                        Contact first name: _{}_\
                    ",
                    data.phone_number,
                    data.first_name,
                );

                // Append the last name
                if let Some(ref last_name) = data.last_name {
                    info += &format!(
                        "\nContact last name: _{}_",
                        last_name,
                    );
                }

                // Append the user ID
                if let Some(ref id) = data.user_id {
                    info += &format!(
                        "\nContact user ID: _{}_",
                        id,
                    );
                }

                info
            },
            MessageKind::Location {
                data,
            } => format!("\
                        Kind: _location_\n\
                        Location longitude: _{}_\n\
                        Location latitude: _{}_\
                    ",
                    data.longitude,
                    data.latitude,
                ),
            MessageKind::Venue {
                data,
            } => {
                // Build generic info
                let mut info = format!("\
                        Kind: _venue_\n\
                        Venue loc longitude: _{}_\n\
                        Venue loc latitude: _{}_\n\
                        Venue title: _{}_\n\
                        Venue address: _{}_\
                    ",
                    data.location.longitude,
                    data.location.latitude,
                    data.title,
                    data.address,
                );

                // Append the foursquare ID
                if let Some(ref id) = data.foursquare_id {
                    info += &format!(
                        "\nVenue foursquare ID: _{}_",
                        id,
                    );
                }

                info
            },
            MessageKind::NewChatMembers {
                data,
            } => {
                // Build generic info
                let mut info = format!("\
                        Kind: _chat members joined_\n\
                        Joined users: _{}_\
                    ",
                    data.len(),
                );

                // Loop through the users
                for (i, user) in data.iter().enumerate() {
                    info += &format!(
                        "\nJoined user #{} ID: _{}_",
                        i,
                        user.id,
                    );
                }

                info
            },
            MessageKind::LeftChatMember {
                data,
            } => format!("\
                        Kind: _chat member left_\n\
                        Left user ID: _{}_\n\
                    ",
                    data.id,
                ),
            MessageKind::NewChatTitle {
                data,
            } => format!("\
                        Kind: _new chat title_\n\
                        New title: _{}_\
                    ",
                    data,
                ),
            MessageKind::NewChatPhoto {
                data,
            } => {
                // Build generic info
                let mut info = format!("\
                        Kind: _new chat photo_\n\
                        Photo file ID: _{}_\n\
                        Photo pixels: _{}x{}_\
                    ",
                    data.file_id,
                    data.width,
                    data.height,
                );

                // Append the file size
                if let Some(ref size) = data.file_size {
                    info += &format!(
                        "\nPhoto size: _{}_",
                        Self::format_file_size(size),
                    );
                }

                info
            },
            MessageKind::DeleteChatPhoto =>
                String::from("Kind: _deleted chat photo_"),
            MessageKind::GroupChatCreated =>
                String::from("Kind: _created group chat_"),
            MessageKind::SupergroupChatCreated =>
                String::from("Kind: _created supergroup chat_"),
            MessageKind::ChannelChatCreated =>
                String::from("Kind: _created channel chat_"),
            MessageKind::MigrateToChatId {
                data,
            } => format!("\
                        Kind: _migrated to chat ID_\n\
                        New chat ID: _{}_\n\
                    ",
                    data,
                ),
            MessageKind::MigrateFromChatId {
                data,
            } => format!("\
                        Kind: _migrated from chat ID_\n\
                        Old chat ID: _{}_\n\
                    ",
                    data,
                ),
            MessageKind::PinnedMessage { .. } =>
                // TODO: describe the pinned message
                String::from("Kind: _pinned message_"),
            MessageKind::Unknown { .. } =>
                String::from("Kind: _?_"),
        }
    }

    /// Format the given timestamp in a humanly readable format.
    pub fn format_timestamp(timestamp: i64) -> String {
        DateTime::<Utc>::from_utc(
            NaiveDateTime::from_timestamp(timestamp, 0),
            Utc,
        ).to_string()
    }
}

impl Action for Id {
    fn cmd(&self) -> &'static str {
        CMD
    }

    fn help(&self) -> &'static str {
        HELP
    }

    fn invoke(&self, msg: &Message, api: &Api) -> Box<Future<Item = (), Error = ()>> {
        // Own the message and API
        let msg = msg.clone();
        let api = api.clone();

        // First send a temporary response, to claim an ID for the answer message
        // TODO: make this timeout configurable
        let response = api.send_timeout(
            msg.text_reply("_Gathering facts..._")
                .parse_mode(ParseMode::Markdown),
            Duration::from_secs(10),
        );

        // Build the ID details message and update the answer
        let response = response.and_then(move |msg_answer| {
            if let Some(msg_answer) = msg_answer {
                // Build a list of info elements to print in the final message
                let mut info = Vec::new();

                // Information about the sender and his message
                info.push(Self::build_user_info(&msg.from, "You"));
                info.push(Self::build_msg_info(&msg, "Your message"));

                // Information about a quoted message by the sender
                if let Some(reply_to) = msg.reply_to_message {
                    info.push(Self::build_msg_channel_post_info(&*reply_to, "Your quoted message"));
                }

                // Information about the answer message and the chat
                info.push(Self::build_msg_info(&msg_answer, "This message"));
                info.push(Self::build_chat_info(&msg.chat, "This chat"));

                // Add information about the bot
                info.push(Self::build_user_info(&msg_answer.from, "Bot"));

                // Send the help message
                api.spawn(
                    msg_answer.edit_text(info.join("\n\n"))
                        .parse_mode(ParseMode::Markdown),
                );
            }

            ok(())
        }).map_err(|_| ());

        Box::new(response)
    }
}
