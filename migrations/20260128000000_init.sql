-- Add migration script here

-- Moderation tables
CREATE TABLE IF NOT EXISTS mod_warnings (
    id BIGSERIAL PRIMARY KEY,
    guild_id BIGINT NOT NULL,
    user_id BIGINT NOT NULL,
    moderator_id BIGINT NOT NULL,
    reason TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS mod_config (
    guild_id BIGINT PRIMARY KEY,
    auto_role_id BIGINT,
    log_channel_id BIGINT
);

CREATE INDEX IF NOT EXISTS idx_warnings_guild_user ON mod_warnings(guild_id, user_id);

-- Redeem tables
CREATE TABLE IF NOT EXISTS redeem_servers (
    id BIGSERIAL PRIMARY KEY,
    channel_id BIGINT NOT NULL,
    guild_id BIGINT NOT NULL UNIQUE,
    games TEXT NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT TRUE
);

CREATE TABLE IF NOT EXISTS redeem_codes (
    id BIGSERIAL PRIMARY KEY,
    game TEXT NOT NULL,
    code TEXT UNIQUE NOT NULL,
    rewards TEXT,
    expiry TEXT,
    created_at BIGINT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_code ON redeem_codes(code);
CREATE INDEX IF NOT EXISTS idx_game ON redeem_codes(game);

-- Reminder table
CREATE TABLE IF NOT EXISTS reminders (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL,
    guild_id BIGINT NOT NULL,
    channel_id BIGINT NOT NULL,
    message TEXT NOT NULL,
    remind_at BIGINT NOT NULL,
    created_at BIGINT NOT NULL,
    is_sent BOOLEAN NOT NULL DEFAULT FALSE
);

CREATE INDEX IF NOT EXISTS idx_remind_at ON reminders(remind_at, is_sent);

-- Forex tables
CREATE TABLE IF NOT EXISTS forex_channels (
    id BIGSERIAL PRIMARY KEY,
    channel_id BIGINT NOT NULL,
    guild_id BIGINT NOT NULL UNIQUE,
    is_active BOOLEAN NOT NULL DEFAULT TRUE
);

CREATE TABLE IF NOT EXISTS forex_news_sent (
    id BIGSERIAL PRIMARY KEY,
    news_id TEXT UNIQUE NOT NULL,
    source TEXT NOT NULL,
    sent_at BIGINT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_forex_news_id ON forex_news_sent(news_id);
