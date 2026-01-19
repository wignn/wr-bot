use rusqlite::{Connection, Result};

#[derive(Debug, Clone)]
pub struct ForexChannel {
    pub id: u64,
    pub channel_id: u64,
    pub guild_id: u64,
    pub is_active: bool,
}

pub struct ForexRepository;

impl ForexRepository {
    pub fn init_tables(conn: &Connection) -> Result<()> {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS forex_channels (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                channel_id INTEGER NOT NULL,
                guild_id INTEGER NOT NULL UNIQUE,
                is_active INTEGER NOT NULL DEFAULT 1
            )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS forex_news_sent (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                news_id TEXT UNIQUE NOT NULL,
                source TEXT NOT NULL,
                sent_at INTEGER NOT NULL
            )",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_forex_news_id ON forex_news_sent(news_id)",
            [],
        )?;

        Ok(())
    }

    pub fn insert_channel(conn: &Connection, guild_id: u64, channel_id: u64) -> Result<()> {
        conn.execute(
            "INSERT OR REPLACE INTO forex_channels (guild_id, channel_id, is_active)
             VALUES (?1, ?2, 1)",
            rusqlite::params![guild_id, channel_id],
        )?;
        Ok(())
    }

    pub fn disable_channel(conn: &Connection, guild_id: u64) -> Result<()> {
        conn.execute(
            "UPDATE forex_channels SET is_active = 0 WHERE guild_id = ?1",
            rusqlite::params![guild_id],
        )?;
        Ok(())
    }

    pub fn enable_channel(conn: &Connection, guild_id: u64) -> Result<()> {
        conn.execute(
            "UPDATE forex_channels SET is_active = 1 WHERE guild_id = ?1",
            rusqlite::params![guild_id],
        )?;
        Ok(())
    }

    pub fn get_active_channels(conn: &Connection) -> Result<Vec<ForexChannel>> {
        let mut stmt = conn.prepare(
            "SELECT id, channel_id, guild_id, is_active FROM forex_channels WHERE is_active = 1"
        )?;

        let channels = stmt.query_map([], |row| {
            Ok(ForexChannel {
                id: row.get(0)?,
                channel_id: row.get(1)?,
                guild_id: row.get(2)?,
                is_active: row.get::<_, i32>(3)? == 1,
            })
        })?;

        channels.collect()
    }

    pub fn get_channel(conn: &Connection, guild_id: u64) -> Result<Option<ForexChannel>> {
        let mut stmt = conn.prepare(
            "SELECT id, channel_id, guild_id, is_active FROM forex_channels WHERE guild_id = ?1"
        )?;

        let mut rows = stmt.query(rusqlite::params![guild_id])?;

        if let Some(row) = rows.next()? {
            Ok(Some(ForexChannel {
                id: row.get(0)?,
                channel_id: row.get(1)?,
                guild_id: row.get(2)?,
                is_active: row.get::<_, i32>(3)? == 1,
            }))
        } else {
            Ok(None)
        }
    }

    pub fn is_news_sent(conn: &Connection, news_id: &str) -> Result<bool> {
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM forex_news_sent WHERE news_id = ?1",
            rusqlite::params![news_id],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    }

    pub fn insert_news(conn: &Connection, news_id: &str, source: &str) -> Result<()> {
        let now = chrono::Utc::now().timestamp();
        conn.execute(
            "INSERT OR IGNORE INTO forex_news_sent (news_id, source, sent_at) VALUES (?1, ?2, ?3)",
            rusqlite::params![news_id, source, now],
        )?;
        Ok(())
    }

    pub fn cleanup_old_news(conn: &Connection, days: i64) -> Result<usize> {
        let cutoff = chrono::Utc::now().timestamp() - (days * 86400);
        let deleted = conn.execute(
            "DELETE FROM forex_news_sent WHERE sent_at < ?1",
            rusqlite::params![cutoff],
        )?;
        Ok(deleted)
    }
}
