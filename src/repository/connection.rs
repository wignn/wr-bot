use rusqlite::{Connection, Result};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::repository::redeem::{verify_tables, RedeemRepository};
use crate::repository::reminder::ReminderRepository;

#[derive(Debug)]
pub struct DbConnection {
    conn: Connection,
}

impl DbConnection {
    pub fn new(db_path: &str) -> Result<Self> {
        let conn = Connection::open(db_path)?;
        RedeemRepository::init_tables(&conn)?;
        ReminderRepository::init_tables(&conn)?;
        verify_tables(&conn)?;
        println!("Database tables initialized and verified");
        Ok(Self { conn })
    }

    pub fn get_connection(&self) -> &Connection {
        &self.conn
    }
}

pub type DbPool = Arc<Mutex<DbConnection>>;

pub fn create_pool(db_path: &str) -> Result<DbPool> {
    let conn = DbConnection::new(db_path)?;
    Ok(Arc::new(Mutex::new(conn)))
}
