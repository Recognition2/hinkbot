use chrono::NaiveDateTime;

#[derive(Queryable)]
pub struct Chat {
    pub telegram_id: i64,
    pub title: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Queryable)]
pub struct User {
    pub telegram_id: i64,
    pub name: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Queryable)]
pub struct ChatUserStats {
    pub chat_id: i64,
    pub user_id: i64,
    pub messages: u32,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}
