use rusqlite::{Connection, Result};

#[derive(Debug, Clone)]
pub struct RedeemServer {
    pub id: u64,
    pub channel_id: u64,
    pub guild_id: u64,
    pub games: String,
    pub is_active: bool,
}

#[derive(Debug, Clone)]
pub struct RedeemCode {
    pub id: u64,
    pub game: String,
    pub code: String,
    pub description: Option<String>,
    pub expiry: Option<String>,
    pub created_at: i64,
}

pub struct RedeemRepository;

impl RedeemRepository {
    pub fn init_tables(conn: &Connection) -> Result<()> {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS redeem_servers (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                channel_id INTEGER NOT NULL,
                guild_id INTEGER NOT NULL UNIQUE,
                games TEXT NOT NULL,
                is_active INTEGER NOT NULL DEFAULT 1
            )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS redeem_codes (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                code TEXT UNIQUE NOT NULL,
                rewards TEXT NOT NULL,
                status TEXT NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_code ON redeem_codes(code)",
            [],
        ).ok();

        Ok(())
    }

    pub fn insert_server(conn: &Connection, guild_id: u64, channel_id: u64, games: &str) -> Result<()> {
        conn.execute(
            "INSERT OR REPLACE INTO redeem_servers (guild_id, channel_id, games, is_active)
             VALUES (?1, ?2, ?3, 1)",
            rusqlite::params![guild_id, channel_id, games],
        )?;
        Ok(())
    }

    pub fn get_active_servers(conn: &Connection, game: &str) -> Result<Vec<RedeemServer>> {
        let mut stmt = conn.prepare(
            "SELECT id, channel_id, guild_id, games, is_active
             FROM redeem_servers
             WHERE is_active = 1"
        )?;

        let servers = stmt
            .query_map([], |row| {
                Ok(RedeemServer {
                    id: row.get(0)?,
                    channel_id: row.get(1)?,
                    guild_id: row.get(2)?,
                    games: row.get(3)?,
                    is_active: row.get(4)?,
                })
            })?
            .filter_map(Result::ok)
            .filter(|server| server.games.contains(game))
            .collect();

        Ok(servers)
    }


    pub fn disable_server(conn: &Connection, guild_id: u64) -> Result<()> {
        conn.execute(
            "UPDATE redeem_servers SET is_active = 0 WHERE guild_id = ?1",
            [guild_id],
        )?;
        Ok(())
    }

    pub fn enable_server(conn: &Connection, guild_id: u64) -> Result<()> {
        conn.execute(
            "UPDATE redeem_servers SET is_active = 1 WHERE guild_id = ?1",
            [guild_id],
        )?;
        Ok(())
    }

    pub fn insert_code(
        conn: &Connection,
        game: &str,
        code: &str,
        description: Option<&str>,
        expiry: Option<&str>,
    ) -> Result<()> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        conn.execute(
            "INSERT OR IGNORE INTO redeem_codes (game, code, description, expiry, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![game, code, description, expiry, now],
        )?;
        Ok(())
    }

    pub fn is_code_sent(conn: &Connection, code: &str) -> Result<bool> {
        let mut stmt = conn.prepare("SELECT COUNT(*) FROM redeem_codes WHERE code = ?1")?;
        let count: i64 = stmt.query_row([code], |row| row.get(0))?;
        Ok(count > 0)
    }
    pub fn get_codes_by_game(conn: &Connection, game: &str) -> Result<Vec<RedeemCode>> {
        let mut stmt = conn.prepare(
            "SELECT id, game, code, description, expiry, created_at
             FROM redeem_codes
             WHERE game = ?1
             ORDER BY created_at DESC
             LIMIT 10"
        )?;

        let codes = stmt
            .query_map([game], |row| {
                Ok(RedeemCode {
                    id: row.get(0)?,
                    game: row.get(1)?,
                    code: row.get(2)?,
                    description: row.get(3)?,
                    expiry: row.get(4)?,
                    created_at: row.get(5)?,
                })
            })?
            .filter_map(Result::ok)
            .collect();

        Ok(codes)
    }

    pub fn delete_expired_codes(conn: &Connection, days_old: i64) -> Result<usize> {
        let cutoff = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64
            - (days_old * 24 * 60 * 60);

        conn.execute(
            "DELETE FROM redeem_codes WHERE created_at < ?1",
            [cutoff],
        )
    }
}

