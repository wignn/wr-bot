use crate::services::music::queue::{MusicQueue, QueuedTrack};
use lavalink_rs::client::LavalinkClient;
use lavalink_rs::model::track::TrackData;
use parking_lot::RwLock;
use serenity::all::{ChannelId, GuildId, Http, UserId};
use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;
use once_cell::sync::OnceCell;

pub type GuildQueues = Arc<RwLock<HashMap<GuildId, MusicQueue>>>;

static GLOBAL_MUSIC_PLAYER: OnceCell<MusicPlayer> = OnceCell::new();
static GLOBAL_HTTP: OnceCell<Arc<Http>> = OnceCell::new();
static BOT_USER_ID: OnceCell<UserId> = OnceCell::new();

pub fn init_global_player(player: MusicPlayer) {
    let _ = GLOBAL_MUSIC_PLAYER.set(player);
}

pub fn init_global_http(http: Arc<Http>) {
    let _ = GLOBAL_HTTP.set(http);
}

pub fn init_bot_user_id(user_id: UserId) {
    let _ = BOT_USER_ID.set(user_id);
}

pub fn get_global_player() -> Option<&'static MusicPlayer> {
    GLOBAL_MUSIC_PLAYER.get()
}

pub fn get_global_http() -> Option<&'static Arc<Http>> {
    GLOBAL_HTTP.get()
}

pub fn get_bot_user_id() -> Option<UserId> {
    BOT_USER_ID.get().copied()
}

#[derive(Clone)]
pub struct MusicPlayer {
    pub lavalink: LavalinkClient,
    pub queues: GuildQueues,
}

impl fmt::Debug for MusicPlayer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MusicPlayer")
            .field("queues", &self.queues)
            .finish()
    }
}

impl MusicPlayer {
    pub fn new(lavalink: LavalinkClient) -> Self {
        Self {
            lavalink,
            queues: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn get_queue(&self, guild_id: GuildId) -> MusicQueue {
        self.queues
            .read()
            .get(&guild_id)
            .cloned()
            .unwrap_or_default()
    }

    pub fn ensure_queue(&self, guild_id: GuildId) {
        let mut queues = self.queues.write();
        queues.entry(guild_id).or_insert_with(MusicQueue::new);
    }

    pub fn add_to_queue(&self, guild_id: GuildId, track: QueuedTrack) {
        let mut queues = self.queues.write();
        let queue = queues.entry(guild_id).or_insert_with(MusicQueue::new);
        queue.add(track);
    }

    pub fn next_track(&self, guild_id: GuildId) -> Option<QueuedTrack> {
        let mut queues = self.queues.write();
        if let Some(queue) = queues.get_mut(&guild_id) {
            queue.next()
        } else {
            None
        }
    }

    pub fn clear_queue(&self, guild_id: GuildId) {
        let mut queues = self.queues.write();
        if let Some(queue) = queues.get_mut(&guild_id) {
            queue.clear();
        }
    }

    pub fn set_text_channel(&self, guild_id: GuildId, channel_id: ChannelId) {
        let mut queues = self.queues.write();
        let queue = queues.entry(guild_id).or_insert_with(MusicQueue::new);
        queue.text_channel_id = Some(channel_id);
    }

    pub fn get_text_channel(&self, guild_id: GuildId) -> Option<ChannelId> {
        self.queues.read().get(&guild_id)?.text_channel_id
    }

    pub fn set_current(&self, guild_id: GuildId, track: Option<QueuedTrack>) {
        let mut queues = self.queues.write();
        if let Some(queue) = queues.get_mut(&guild_id) {
            queue.current = track;
        }
    }

    pub fn get_current(&self, guild_id: GuildId) -> Option<QueuedTrack> {
        self.queues.read().get(&guild_id)?.current.clone()
    }

    pub fn set_volume(&self, guild_id: GuildId, volume: u8) {
        let mut queues = self.queues.write();
        if let Some(queue) = queues.get_mut(&guild_id) {
            queue.volume = volume.min(150);
        }
    }

    pub fn get_volume(&self, guild_id: GuildId) -> u8 {
        self.queues
            .read()
            .get(&guild_id)
            .map(|q| q.volume)
            .unwrap_or(100)
    }

    pub fn set_paused(&self, guild_id: GuildId, paused: bool) {
        let mut queues = self.queues.write();
        if let Some(queue) = queues.get_mut(&guild_id) {
            queue.is_paused = paused;
        }
    }

    pub fn is_paused(&self, guild_id: GuildId) -> bool {
        self.queues
            .read()
            .get(&guild_id)
            .map(|q| q.is_paused)
            .unwrap_or(false)
    }

    pub fn set_loop(&self, guild_id: GuildId, looping: bool) {
        let mut queues = self.queues.write();
        if let Some(queue) = queues.get_mut(&guild_id) {
            queue.is_looping = looping;
        }
    }

    pub fn is_looping(&self, guild_id: GuildId) -> bool {
        self.queues
            .read()
            .get(&guild_id)
            .map(|q| q.is_looping)
            .unwrap_or(false)
    }

    pub fn shuffle_queue(&self, guild_id: GuildId) {
        let mut queues = self.queues.write();
        if let Some(queue) = queues.get_mut(&guild_id) {
            queue.shuffle();
        }
    }

    pub fn remove_from_queue(&self, guild_id: GuildId, index: usize) -> Option<QueuedTrack> {
        let mut queues = self.queues.write();
        queues.get_mut(&guild_id)?.remove(index)
    }

    pub fn get_player_context(
        &self,
        guild_id: GuildId,
    ) -> Option<lavalink_rs::player_context::PlayerContext> {
        let lavalink_guild_id = lavalink_rs::model::GuildId(guild_id.get());
        self.lavalink.get_player_context(lavalink_guild_id)
    }

    pub async fn create_player_with_connection(
        &self,
        guild_id: GuildId,
        connection_info: lavalink_rs::model::player::ConnectionInfo,
    ) -> Result<lavalink_rs::player_context::PlayerContext, String> {
        let lavalink_guild_id = lavalink_rs::model::GuildId(guild_id.get());
        
        if let Some(ctx) = self.lavalink.get_player_context(lavalink_guild_id) {
            return Ok(ctx);
        }

        self.lavalink
            .create_player_context(lavalink_guild_id, connection_info)
            .await
            .map_err(|e| format!("Failed to create player: {}", e))
    }

    pub async fn search_tracks(&self, guild_id: GuildId, query: &str) -> Result<Vec<TrackData>, String> {
        let search_query = if query.starts_with("http://") || query.starts_with("https://") {
            query.to_string()
        } else {
            format!("spsearch:{}", query)
        };

        println!("[MUSIC] Searching with query: {}", search_query);

        let lavalink_guild_id = lavalink_rs::model::GuildId(guild_id.get());

        match self
            .lavalink
            .load_tracks(lavalink_guild_id, &search_query)
            .await
        {
            Ok(loaded) => {
                use lavalink_rs::model::track::TrackLoadData;
                match loaded.data {
                    Some(TrackLoadData::Track(track)) => Ok(vec![track]),
                    Some(TrackLoadData::Playlist(playlist)) => Ok(playlist.tracks),
                    Some(TrackLoadData::Search(tracks)) => {
                        if tracks.is_empty() && !query.starts_with("http") {
                            println!("[DEBUG] Spotify returned no results, trying YouTube...");
                            let yt_query = format!("ytsearch:{}", query);
                            match self.lavalink.load_tracks(lavalink_guild_id, &yt_query).await {
                                Ok(yt_loaded) => match yt_loaded.data {
                                    Some(TrackLoadData::Track(t)) => Ok(vec![t]),
                                    Some(TrackLoadData::Search(t)) => Ok(t),
                                    Some(TrackLoadData::Playlist(p)) => Ok(p.tracks),
                                    _ => Ok(vec![]),
                                },
                                Err(_) => Ok(vec![]),
                            }
                        } else {
                            Ok(tracks)
                        }
                    }
                    Some(TrackLoadData::Error(err)) => {
                        println!(
                            "[DEBUG] Spotify search error: {}, trying YouTube...",
                            err.message
                        );
                        // Fallback to YouTube on error
                        let yt_query = format!("ytsearch:{}", query);
                        match self.lavalink.load_tracks(lavalink_guild_id, &yt_query).await {
                            Ok(yt_loaded) => match yt_loaded.data {
                                Some(TrackLoadData::Track(t)) => Ok(vec![t]),
                                Some(TrackLoadData::Search(t)) => Ok(t),
                                Some(TrackLoadData::Playlist(p)) => Ok(p.tracks),
                                _ => Err(err.message),
                            },
                            Err(e) => Err(format!("Search failed: {}", e)),
                        }
                    }
                    None => Ok(vec![]),
                }
            }
            Err(e) => Err(format!("Failed to search: {}", e)),
        }
    }
}
