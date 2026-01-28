use chrono::Utc;
use sqlx::PgPool;

/// Warning record for a user
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Warning {
    pub id: i64,
    pub guild_id: i64,
    pub user_id: i64,
    pub moderator_id: i64,
    pub reason: String,
    pub created_at: chrono::DateTime<Utc>,
}

/// Moderation config for a guild (auto-role, log channel)
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ModConfig {
    pub guild_id: i64,
    pub auto_role_id: Option<i64>,
    pub log_channel_id: Option<i64>,
}

pub struct ModerationRepository;

impl ModerationRepository {
    // ==================== WARNINGS ====================

    /// Add a warning to a user
    pub async fn add_warning(
        pool: &PgPool,
        guild_id: u64,
        user_id: u64,
        moderator_id: u64,
        reason: &str,
    ) -> Result<i64, sqlx::Error> {
        let result = sqlx::query_scalar!(
            r#"
            INSERT INTO mod_warnings (guild_id, user_id, moderator_id, reason, created_at)
            VALUES ($1, $2, $3, $4, NOW())
            RETURNING id
            "#,
            guild_id as i64,
            user_id as i64,
            moderator_id as i64,
            reason,
        )
        .fetch_one(pool)
        .await?;

        Ok(result)
    }

    /// Get all warnings for a user in a guild
    pub async fn get_warnings(
        pool: &PgPool,
        guild_id: u64,
        user_id: u64,
    ) -> Result<Vec<Warning>, sqlx::Error> {
        let warnings = sqlx::query_as!(
            Warning,
            r#"
            SELECT id, guild_id, user_id, moderator_id, reason, created_at
            FROM mod_warnings
            WHERE guild_id = $1 AND user_id = $2
            ORDER BY created_at DESC
            "#,
            guild_id as i64,
            user_id as i64,
        )
        .fetch_all(pool)
        .await?;

        Ok(warnings)
    }

    /// Get warning count for a user
    pub async fn get_warning_count(
        pool: &PgPool,
        guild_id: u64,
        user_id: u64,
    ) -> Result<i64, sqlx::Error> {
        let count = sqlx::query_scalar!(
            r#"SELECT COUNT(*) as "count!" FROM mod_warnings WHERE guild_id = $1 AND user_id = $2"#,
            guild_id as i64,
            user_id as i64,
        )
        .fetch_one(pool)
        .await?;

        Ok(count)
    }

    /// Clear all warnings for a user
    pub async fn clear_warnings(
        pool: &PgPool,
        guild_id: u64,
        user_id: u64,
    ) -> Result<u64, sqlx::Error> {
        let result = sqlx::query!(
            "DELETE FROM mod_warnings WHERE guild_id = $1 AND user_id = $2",
            guild_id as i64,
            user_id as i64,
        )
        .execute(pool)
        .await?;

        Ok(result.rows_affected())
    }

    /// Delete a specific warning by ID
    pub async fn delete_warning(
        pool: &PgPool,
        warning_id: i64,
        guild_id: u64,
    ) -> Result<bool, sqlx::Error> {
        let result = sqlx::query!(
            "DELETE FROM mod_warnings WHERE id = $1 AND guild_id = $2",
            warning_id,
            guild_id as i64,
        )
        .execute(pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    // ==================== MOD CONFIG ====================

    /// Get mod config for a guild
    pub async fn get_config(
        pool: &PgPool,
        guild_id: u64,
    ) -> Result<Option<ModConfig>, sqlx::Error> {
        let config = sqlx::query_as!(
            ModConfig,
            "SELECT guild_id, auto_role_id, log_channel_id FROM mod_config WHERE guild_id = $1",
            guild_id as i64,
        )
        .fetch_optional(pool)
        .await?;

        Ok(config)
    }

    /// Set auto-role for a guild
    pub async fn set_auto_role(
        pool: &PgPool,
        guild_id: u64,
        role_id: u64,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
            INSERT INTO mod_config (guild_id, auto_role_id)
            VALUES ($1, $2)
            ON CONFLICT(guild_id) DO UPDATE SET auto_role_id = EXCLUDED.auto_role_id
            "#,
            guild_id as i64,
            role_id as i64,
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Disable auto-role for a guild
    pub async fn disable_auto_role(pool: &PgPool, guild_id: u64) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "UPDATE mod_config SET auto_role_id = NULL WHERE guild_id = $1",
            guild_id as i64,
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Set log channel for a guild
    pub async fn set_log_channel(
        pool: &PgPool,
        guild_id: u64,
        channel_id: u64,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
            INSERT INTO mod_config (guild_id, log_channel_id)
            VALUES ($1, $2)
            ON CONFLICT(guild_id) DO UPDATE SET log_channel_id = EXCLUDED.log_channel_id
            "#,
            guild_id as i64,
            channel_id as i64,
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Disable logging for a guild
    pub async fn disable_logging(pool: &PgPool, guild_id: u64) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "UPDATE mod_config SET log_channel_id = NULL WHERE guild_id = $1",
            guild_id as i64,
        )
        .execute(pool)
        .await?;

        Ok(())
    }
}
