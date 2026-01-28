use sqlx::PgPool;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Reminder {
    pub id: i64,
    pub user_id: i64,
    pub guild_id: i64,
    pub channel_id: i64,
    pub message: String,
    pub remind_at: i64,
    pub created_at: i64,
    pub is_sent: bool,
}

pub struct ReminderRepository;

impl ReminderRepository {
    pub async fn insert_reminder(
        pool: &PgPool,
        user_id: u64,
        guild_id: u64,
        channel_id: u64,
        message: &str,
        remind_at: i64,
    ) -> Result<i64, sqlx::Error> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let id = sqlx::query_scalar!(
            r#"
            INSERT INTO reminders (user_id, guild_id, channel_id, message, remind_at, created_at, is_sent)
            VALUES ($1, $2, $3, $4, $5, $6, FALSE)
            RETURNING id
            "#,
            user_id as i64,
            guild_id as i64,
            channel_id as i64,
            message,
            remind_at,
            now,
        )
        .fetch_one(pool)
        .await?;

        Ok(id)
    }

    pub async fn get_pending_reminders(pool: &PgPool) -> Result<Vec<Reminder>, sqlx::Error> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let reminders = sqlx::query_as!(
            Reminder,
            r#"
            SELECT id, user_id, guild_id, channel_id, message, remind_at, created_at, is_sent
            FROM reminders
            WHERE is_sent = FALSE AND remind_at <= $1
            ORDER BY remind_at ASC
            "#,
            now,
        )
        .fetch_all(pool)
        .await?;

        Ok(reminders)
    }

    pub async fn mark_as_sent(pool: &PgPool, reminder_id: i64) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "UPDATE reminders SET is_sent = TRUE WHERE id = $1",
            reminder_id,
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn get_user_reminders(
        pool: &PgPool,
        user_id: u64,
    ) -> Result<Vec<Reminder>, sqlx::Error> {
        let reminders = sqlx::query_as!(
            Reminder,
            r#"
            SELECT id, user_id, guild_id, channel_id, message, remind_at, created_at, is_sent
            FROM reminders
            WHERE user_id = $1 AND is_sent = FALSE
            ORDER BY remind_at ASC
            LIMIT 10
            "#,
            user_id as i64,
        )
        .fetch_all(pool)
        .await?;

        Ok(reminders)
    }

    pub async fn delete_reminder(
        pool: &PgPool,
        reminder_id: i64,
        user_id: u64,
    ) -> Result<bool, sqlx::Error> {
        let result = sqlx::query!(
            "DELETE FROM reminders WHERE id = $1 AND user_id = $2",
            reminder_id,
            user_id as i64,
        )
        .execute(pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    pub async fn cleanup_sent_reminders(pool: &PgPool, days_old: i64) -> Result<u64, sqlx::Error> {
        let cutoff = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64
            - (days_old * 24 * 60 * 60);

        let result = sqlx::query!(
            "DELETE FROM reminders WHERE is_sent = TRUE AND created_at < $1",
            cutoff,
        )
        .execute(pool)
        .await?;

        Ok(result.rows_affected())
    }
}
