CREATE TABLE user (
  telegram_id   BIGINT      PRIMARY KEY,
  first_name    TEXT        NOT NULL,
  last_name     TEXT        DEFAULT NULL,
  created_at    DATETIME    NOT NULL
    DEFAULT CURRENT_TIMESTAMP,
  updated_at    DATETIME    NOT NULL
    DEFAULT CURRENT_TIMESTAMP
    ON UPDATE CURRENT_TIMESTAMP
);
