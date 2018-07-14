CREATE TABLE user (
  telegram_id   BIGINT      PRIMARY KEY,
  name          TEXT        DEFAULT NULL,
  created_at    DATETIME    NOT NULL        DEFAULT CURRENT_TIMESTAMP,
  updated_at    DATETIME    NOT NULL        DEFAULT CURRENT_TIMESTAMP
);
