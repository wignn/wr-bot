use sqlx::PgPool;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct RedeemServer {
    pub id: i64,
    pub channel_id: i64,
    pub guild_id: i64,
    pub games: String,
    pub is_active: bool,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct RedeemCode {
    pub id: i64,
    pub game: String,
    pub code: String,
    pub rewards: Option<String>,
    pub expiry: Option<String>,
    pub created_at: i64,
}

pub struct RedeemRepository;

impl RedeemRepository {
    pub async fn insert_server(
        pool: &PgPool,
        guild_id: u64,
        channel_id: u64,
        games: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
            INSERT INTO redeem_servers (guild_id, channel_id, games, is_active)
            VALUES ($1, $2, $3, TRUE)
            ON CONFLICT(guild_id) DO UPDATE SET channel_id = $2, games = $3, is_active = TRUE
            "#,
            guild_id as i64,
            channel_id as i64,
            games,
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn get_active_servers(
        pool: &PgPool,
        game: &str,
    ) -> Result<Vec<RedeemServer>, sqlx::Error> {
        let servers = sqlx::query_as!(
            RedeemServer,
            r#"
            SELECT id, channel_id, guild_id, games, is_active
            FROM redeem_servers
            WHERE is_active = TRUE AND games LIKE '%' || $1 || '%'
            "#,
            game,
        )
        .fetch_all(pool)
        .await?;

        Ok(servers)
    }

    pub async fn disable_server(pool: &PgPool, guild_id: u64) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "UPDATE redeem_servers SET is_active = FALSE WHERE guild_id = $1",
            guild_id as i64,
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn enable_server(pool: &PgPool, guild_id: u64) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "UPDATE redeem_servers SET is_active = TRUE WHERE guild_id = $1",
            guild_id as i64,
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn insert_code(
        pool: &PgPool,
        game: &str,
        code: &str,
        rewards: Option<&str>,
        expiry: Option<&str>,
    ) -> Result<(), sqlx::Error> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        sqlx::query!(
            r#"
            INSERT INTO redeem_codes (game, code, rewards, expiry, created_at)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT(code) DO NOTHING
            "#,
            game,
            code,
            rewards,
            expiry,
            now,
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn is_code_sent(pool: &PgPool, code: &str) -> Result<bool, sqlx::Error> {
        let count = sqlx::query_scalar!(
            r#"SELECT COUNT(*) as "count!" FROM redeem_codes WHERE code = $1"#,
            code,
        )
        .fetch_one(pool)
        .await?;

        Ok(count > 0)
    }

    pub async fn get_codes_by_game(
        pool: &PgPool,
        game: &str,
    ) -> Result<Vec<RedeemCode>, sqlx::Error> {
        let codes = sqlx::query_as!(
            RedeemCode,
            r#"
            SELECT id, game, code, rewards, expiry, created_at
            FROM redeem_codes
            WHERE game = $1
            ORDER BY created_at DESC
            LIMIT 10
            "#,
            game,
        )
        .fetch_all(pool)
        .await?;

        Ok(codes)
    }

    pub async fn delete_expired_codes(pool: &PgPool, days_old: i64) -> Result<u64, sqlx::Error> {
        let cutoff = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64
            - (days_old * 24 * 60 * 60);

        let result = sqlx::query!("DELETE FROM redeem_codes WHERE created_at < $1", cutoff,)
            .execute(pool)
            .await?;

        Ok(result.rows_affected())
    }
}
