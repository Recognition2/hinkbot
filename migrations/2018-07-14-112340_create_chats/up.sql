CREATE TABLE chat (
  telegram_id   BIGINT      PRIMARY KEY,
  title         TEXT        NOT NULL,
  created_at    DATETIME    NOT NULL        DEFAULT CURRENT_TIMESTAMP,
  updated_at    DATETIME    NOT NULL        DEFAULT CURRENT_TIMESTAMP
);
