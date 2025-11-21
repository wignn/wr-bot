pub mod general;
pub mod admin;
pub mod ping;
pub mod ai;
pub mod sys;
pub mod redeem;
pub mod reminder;
pub mod qr;

use poise::serenity_prelude::UserId;
use std::collections::HashSet;
use crate::repository::DbPool;

#[derive(Clone)]
pub struct Data {
    pub owners: HashSet<UserId>,
    pub db: DbPool,
}