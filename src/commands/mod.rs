pub mod general;
pub mod admin;
pub mod ping;
pub mod ai;

use poise::serenity_prelude::UserId;
use std::collections::HashSet;

#[derive(Clone)]
pub struct Data {
    pub owners: HashSet<UserId>,
}