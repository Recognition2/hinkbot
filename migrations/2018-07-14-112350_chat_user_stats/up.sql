CREATE TABLE chat_user_stats (
  id            INT         AUTO_INCREMENT  PRIMARY KEY,
  chat_id       BIGINT      NOT NULL,
  user_id       BIGINT      NOT NULL,
  messages      INT         NOT NULL        DEFAULT 0,
  created_at    DATETIME    NOT NULL        DEFAULT CURRENT_TIMESTAMP,
  updated_at    DATETIME    NOT NULL        DEFAULT CURRENT_TIMESTAMP,

  FOREIGN KEY (chat_id)
    REFERENCES chat(telegram_id)
    ON DELETE CASCADE,
  FOREIGN KEY (user_id)
    REFERENCES user(telegram_id)
    ON DELETE CASCADE
);
