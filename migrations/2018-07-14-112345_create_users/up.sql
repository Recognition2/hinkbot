CREATE TABLE user (
  telegram_id   BIGINT      PRIMARY KEY,
  first_name    TEXT        DEFAULT NULL,
  last_name     TEXT        DEFAULT NULL,
  created_at    DATETIME    NOT NULL        DEFAULT CURRENT_TIMESTAMP,
  updated_at    DATETIME    NOT NULL        DEFAULT CURRENT_TIMESTAMP
);
