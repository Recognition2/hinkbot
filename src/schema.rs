table! {
    chat (telegram_id) {
        telegram_id -> Integer,
        title -> Text,
        created_at -> Datetime,
        updated_at -> Datetime,
    }
}

table! {
    chat_user_stats (id) {
        id -> Integer,
        chat_id -> Integer,
        user_id -> Integer,
        messages -> Integer,
        created_at -> Datetime,
        updated_at -> Datetime,
    }
}

table! {
    user (telegram_id) {
        telegram_id -> Integer,
        name -> Nullable<Text>,
        created_at -> Datetime,
        updated_at -> Datetime,
    }
}

joinable!(chat_user_stats -> chat (chat_id));
joinable!(chat_user_stats -> user (user_id));

allow_tables_to_appear_in_same_query!(
    chat,
    chat_user_stats,
    user,
);
