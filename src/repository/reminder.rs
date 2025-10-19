use rusqlite::{Connection, Result};

#[derive(Debug, Clone)]
pub struct Reminder {
    pub id: u64,
    pub user_id: u64,
    pub guild_id: u64,
    pub channel_id: u64,
    pub message: String,
    pub remind_at: i64,
    pub created_at: i64,
    pub is_sent: bool,
}

pub struct ReminderRepository;

impl ReminderRepository {
    pub fn init_tables(conn: &Connection) -> Result<()> {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS reminders (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                user_id INTEGER NOT NULL,
                guild_id INTEGER NOT NULL,
                channel_id INTEGER NOT NULL,
                message TEXT NOT NULL,
                remind_at INTEGER NOT NULL,
                created_at INTEGER NOT NULL,
                is_sent INTEGER NOT NULL DEFAULT 0
            )",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_remind_at
             ON reminders(remind_at, is_sent)",
            [],
        )?;

        Ok(())
    }

    pub fn insert_reminder(
        conn: &Connection,
        user_id: u64,
        guild_id: u64,
        channel_id: u64,
        message: &str,
        remind_at: i64,
    ) -> Result<u64> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        conn.execute(
            "INSERT INTO reminders (user_id, guild_id, channel_id, message, remind_at, created_at, is_sent)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0)",
            rusqlite::params![user_id, guild_id, channel_id, message, remind_at, now],
        )?;

        Ok(conn.last_insert_rowid() as u64)
    }

    pub fn get_pending_reminders(conn: &Connection) -> Result<Vec<Reminder>> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let mut stmt = conn.prepare(
            "SELECT id, user_id, guild_id, channel_id, message, remind_at, created_at, is_sent
             FROM reminders
             WHERE is_sent = 0 AND remind_at <= ?1
             ORDER BY remind_at ASC"
        )?;

        let reminders = stmt
            .query_map([now], |row| {
                Ok(Reminder {
                    id: row.get(0)?,
                    user_id: row.get(1)?,
                    guild_id: row.get(2)?,
                    channel_id: row.get(3)?,
                    message: row.get(4)?,
                    remind_at: row.get(5)?,
                    created_at: row.get(6)?,
                    is_sent: row.get(7)?,
                })
            })?
            .filter_map(Result::ok)
            .collect();

        Ok(reminders)
    }

    pub fn mark_as_sent(conn: &Connection, reminder_id: u64) -> Result<()> {
        conn.execute(
            "UPDATE reminders SET is_sent = 1 WHERE id = ?1",
            [reminder_id],
        )?;
        Ok(())
    }

    pub fn get_user_reminders(conn: &Connection, user_id: u64) -> Result<Vec<Reminder>> {
        let mut stmt = conn.prepare(
            "SELECT id, user_id, guild_id, channel_id, message, remind_at, created_at, is_sent
             FROM reminders
             WHERE user_id = ?1 AND is_sent = 0
             ORDER BY remind_at ASC
             LIMIT 10"
        )?;

        let reminders = stmt
            .query_map([user_id], |row| {
                Ok(Reminder {
                    id: row.get(0)?,
                    user_id: row.get(1)?,
                    guild_id: row.get(2)?,
                    channel_id: row.get(3)?,
                    message: row.get(4)?,
                    remind_at: row.get(5)?,
                    created_at: row.get(6)?,
                    is_sent: row.get(7)?,
                })
            })?
            .filter_map(Result::ok)
            .collect();

        Ok(reminders)
    }

    pub fn delete_reminder(conn: &Connection, reminder_id: u64, user_id: u64) -> Result<bool> {
        let affected = conn.execute(
            "DELETE FROM reminders WHERE id = ?1 AND user_id = ?2",
            rusqlite::params![reminder_id, user_id],
        )?;
        Ok(affected > 0)
    }

    pub fn cleanup_sent_reminders(conn: &Connection, days_old: i64) -> Result<usize> {
        let cutoff = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64
            - (days_old * 24 * 60 * 60);

        conn.execute(
            "DELETE FROM reminders WHERE is_sent = 1 AND created_at < ?1",
            [cutoff],
        )
    }
}

