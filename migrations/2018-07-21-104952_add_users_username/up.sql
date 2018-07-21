ALTER TABLE user
ADD COLUMN
    username TEXT DEFAULT NULL
    AFTER telegram_id;
