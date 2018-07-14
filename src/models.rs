use chrono::NaiveDateTime;

#[derive(Queryable)]
pub struct Chat {
    pub telegram_id: i32,
    pub title: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Queryable)]
pub struct User {
    pub telegram_id: i32,
    pub name: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Queryable)]
pub struct ChatUserStats {
    pub chat_id: i32,
    pub user_id: i32,
    pub messages: u32,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}
