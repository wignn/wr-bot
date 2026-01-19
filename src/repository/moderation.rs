use chrono::Utc;
use rusqlite::{params, Connection, Result};

/// Warning record for a user
#[derive(Debug, Clone)]
pub struct Warning {
    pub id: i64,
    pub guild_id: u64,
    pub user_id: u64,
    pub moderator_id: u64,
    pub reason: String,
    pub created_at: String,
}

/// Moderation config for a guild (auto-role, log channel)
#[derive(Debug, Clone)]
pub struct ModConfig {
    pub guild_id: u64,
    pub auto_role_id: Option<u64>,
    pub log_channel_id: Option<u64>,
}

pub struct ModerationRepository;

impl ModerationRepository {
    /// Initialize moderation tables
    pub fn init_tables(conn: &Connection) -> Result<()> {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS mod_warnings (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                guild_id INTEGER NOT NULL,
                user_id INTEGER NOT NULL,
                moderator_id INTEGER NOT NULL,
                reason TEXT NOT NULL,
                created_at TEXT NOT NULL
            )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS mod_config (
                guild_id INTEGER PRIMARY KEY,
                auto_role_id INTEGER,
                log_channel_id INTEGER
            )",
            [],
        )?;

        // Create indexes for faster lookups
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_warnings_guild_user ON mod_warnings(guild_id, user_id)",
            [],
        )?;

        Ok(())
    }

    // ==================== WARNINGS ====================

    /// Add a warning to a user
    pub fn add_warning(
        conn: &Connection,
        guild_id: u64,
        user_id: u64,
        moderator_id: u64,
        reason: &str,
    ) -> Result<i64> {
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO mod_warnings (guild_id, user_id, moderator_id, reason, created_at) 
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                guild_id as i64,
                user_id as i64,
                moderator_id as i64,
                reason,
                now
            ],
        )?;
        Ok(conn.last_insert_rowid())
    }

    /// Get all warnings for a user in a guild
    pub fn get_warnings(conn: &Connection, guild_id: u64, user_id: u64) -> Result<Vec<Warning>> {
        let mut stmt = conn.prepare(
            "SELECT id, guild_id, user_id, moderator_id, reason, created_at 
             FROM mod_warnings 
             WHERE guild_id = ?1 AND user_id = ?2 
             ORDER BY created_at DESC",
        )?;

        let warnings = stmt.query_map(params![guild_id as i64, user_id as i64], |row| {
            Ok(Warning {
                id: row.get(0)?,
                guild_id: row.get::<_, i64>(1)? as u64,
                user_id: row.get::<_, i64>(2)? as u64,
                moderator_id: row.get::<_, i64>(3)? as u64,
                reason: row.get(4)?,
                created_at: row.get(5)?,
            })
        })?;

        warnings.collect()
    }

    /// Get warning count for a user
    pub fn get_warning_count(conn: &Connection, guild_id: u64, user_id: u64) -> Result<i64> {
        conn.query_row(
            "SELECT COUNT(*) FROM mod_warnings WHERE guild_id = ?1 AND user_id = ?2",
            params![guild_id as i64, user_id as i64],
            |row| row.get(0),
        )
    }

    /// Clear all warnings for a user
    pub fn clear_warnings(conn: &Connection, guild_id: u64, user_id: u64) -> Result<usize> {
        conn.execute(
            "DELETE FROM mod_warnings WHERE guild_id = ?1 AND user_id = ?2",
            params![guild_id as i64, user_id as i64],
        )
    }

    /// Delete a specific warning by ID
    pub fn delete_warning(conn: &Connection, warning_id: i64, guild_id: u64) -> Result<bool> {
        let affected = conn.execute(
            "DELETE FROM mod_warnings WHERE id = ?1 AND guild_id = ?2",
            params![warning_id, guild_id as i64],
        )?;
        Ok(affected > 0)
    }

    // ==================== MOD CONFIG ====================

    /// Get mod config for a guild
    pub fn get_config(conn: &Connection, guild_id: u64) -> Result<Option<ModConfig>> {
        let result = conn.query_row(
            "SELECT guild_id, auto_role_id, log_channel_id FROM mod_config WHERE guild_id = ?1",
            params![guild_id as i64],
            |row| {
                Ok(ModConfig {
                    guild_id: row.get::<_, i64>(0)? as u64,
                    auto_role_id: row.get::<_, Option<i64>>(1)?.map(|v| v as u64),
                    log_channel_id: row.get::<_, Option<i64>>(2)?.map(|v| v as u64),
                })
            },
        );

        match result {
            Ok(config) => Ok(Some(config)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Set auto-role for a guild
    pub fn set_auto_role(conn: &Connection, guild_id: u64, role_id: u64) -> Result<()> {
        conn.execute(
            "INSERT INTO mod_config (guild_id, auto_role_id) VALUES (?1, ?2)
             ON CONFLICT(guild_id) DO UPDATE SET auto_role_id = excluded.auto_role_id",
            params![guild_id as i64, role_id as i64],
        )?;
        Ok(())
    }

    /// Disable auto-role for a guild
    pub fn disable_auto_role(conn: &Connection, guild_id: u64) -> Result<()> {
        conn.execute(
            "UPDATE mod_config SET auto_role_id = NULL WHERE guild_id = ?1",
            params![guild_id as i64],
        )?;
        Ok(())
    }

    /// Set log channel for a guild
    pub fn set_log_channel(conn: &Connection, guild_id: u64, channel_id: u64) -> Result<()> {
        conn.execute(
            "INSERT INTO mod_config (guild_id, log_channel_id) VALUES (?1, ?2)
             ON CONFLICT(guild_id) DO UPDATE SET log_channel_id = excluded.log_channel_id",
            params![guild_id as i64, channel_id as i64],
        )?;
        Ok(())
    }

    /// Disable logging for a guild
    pub fn disable_logging(conn: &Connection, guild_id: u64) -> Result<()> {
        conn.execute(
            "UPDATE mod_config SET log_channel_id = NULL WHERE guild_id = ?1",
            params![guild_id as i64],
        )?;
        Ok(())
    }
}
