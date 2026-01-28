use sqlx::PgPool;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ForexChannel {
    pub id: i64,
    pub channel_id: i64,
    pub guild_id: i64,
    pub is_active: bool,
}

pub struct ForexRepository;

impl ForexRepository {
    pub async fn insert_channel(
        pool: &PgPool,
        guild_id: u64,
        channel_id: u64,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
            INSERT INTO forex_channels (guild_id, channel_id, is_active)
            VALUES ($1, $2, TRUE)
            ON CONFLICT(guild_id) DO UPDATE SET channel_id = $2, is_active = TRUE
            "#,
            guild_id as i64,
            channel_id as i64,
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn disable_channel(pool: &PgPool, guild_id: u64) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "UPDATE forex_channels SET is_active = FALSE WHERE guild_id = $1",
            guild_id as i64,
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn enable_channel(pool: &PgPool, guild_id: u64) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "UPDATE forex_channels SET is_active = TRUE WHERE guild_id = $1",
            guild_id as i64,
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn get_active_channels(pool: &PgPool) -> Result<Vec<ForexChannel>, sqlx::Error> {
        let channels = sqlx::query_as!(
            ForexChannel,
            "SELECT id, channel_id, guild_id, is_active FROM forex_channels WHERE is_active = TRUE"
        )
        .fetch_all(pool)
        .await?;

        Ok(channels)
    }

    pub async fn get_channel(
        pool: &PgPool,
        guild_id: u64,
    ) -> Result<Option<ForexChannel>, sqlx::Error> {
        let channel = sqlx::query_as!(
            ForexChannel,
            "SELECT id, channel_id, guild_id, is_active FROM forex_channels WHERE guild_id = $1",
            guild_id as i64,
        )
        .fetch_optional(pool)
        .await?;

        Ok(channel)
    }

    pub async fn is_news_sent(pool: &PgPool, news_id: &str) -> Result<bool, sqlx::Error> {
        let count = sqlx::query_scalar!(
            r#"SELECT COUNT(*) as "count!" FROM forex_news_sent WHERE news_id = $1"#,
            news_id,
        )
        .fetch_one(pool)
        .await?;

        Ok(count > 0)
    }

    pub async fn insert_news(
        pool: &PgPool,
        news_id: &str,
        source: &str,
    ) -> Result<(), sqlx::Error> {
        let now = chrono::Utc::now().timestamp();
        sqlx::query!(
            r#"
            INSERT INTO forex_news_sent (news_id, source, sent_at)
            VALUES ($1, $2, $3)
            ON CONFLICT(news_id) DO NOTHING
            "#,
            news_id,
            source,
            now,
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn cleanup_old_news(pool: &PgPool, days: i64) -> Result<u64, sqlx::Error> {
        let cutoff = chrono::Utc::now().timestamp() - (days * 86400);
        let result = sqlx::query!("DELETE FROM forex_news_sent WHERE sent_at < $1", cutoff,)
            .execute(pool)
            .await?;

        Ok(result.rows_affected())
    }
}
