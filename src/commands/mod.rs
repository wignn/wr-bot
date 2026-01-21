pub mod admin;
pub mod ai;
pub mod forex;
pub mod general;
pub mod moderation;
pub mod music;
pub mod ping;
pub mod price;
pub mod redeem;
pub mod sys;

use crate::repository::DbPool;
use crate::services::music::MusicPlayer;
use crate::services::youtube::YouTubeSearch;
use poise::serenity_prelude::UserId;
use songbird::Songbird;
use std::collections::HashSet;
use std::sync::Arc;

#[derive(Clone)]
pub struct Data {
    pub owners: HashSet<UserId>,
    pub db: DbPool,
    pub music_player: Option<MusicPlayer>,
    pub songbird: Arc<Songbird>,
    pub youtube_search: Option<YouTubeSearch>,
}

impl std::fmt::Debug for Data {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Data")
            .field("owners", &self.owners)
            .field("db", &self.db)
            .field("music_player", &self.music_player)
            .field("songbird", &"Arc<Songbird>")
            .field("youtube_search", &self.youtube_search.is_some())
            .finish()
    }
}
