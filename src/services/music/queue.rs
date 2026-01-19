use lavalink_rs::model::track::TrackData;
use serenity::all::ChannelId;
use std::collections::VecDeque;

#[derive(Debug, Clone)]
pub struct MusicQueue {
    pub tracks: VecDeque<QueuedTrack>,
    pub current: Option<QueuedTrack>,
    pub volume: u8,
    pub is_looping: bool,
    pub is_paused: bool,
    pub is_autoplay: bool,
    pub last_track_title: Option<String>,
    pub last_video_id: Option<String>,
    pub played_video_ids: VecDeque<String>, // History (max 20)
    pub text_channel_id: Option<ChannelId>,
}

#[derive(Debug, Clone)]
pub struct QueuedTrack {
    pub track: TrackData,
    pub requester_id: u64,
    pub requester_name: String,
}

impl Default for MusicQueue {
    fn default() -> Self {
        Self::new()
    }
}

impl MusicQueue {
    pub fn new() -> Self {
        Self {
            tracks: VecDeque::new(),
            current: None,
            volume: 100,
            is_looping: false,
            is_paused: false,
            is_autoplay: false,
            last_track_title: None,
            last_video_id: None,
            played_video_ids: VecDeque::with_capacity(20),
            text_channel_id: None,
        }
    }

    pub fn add(&mut self, track: QueuedTrack) {
        self.tracks.push_back(track);
    }

    pub fn next(&mut self) -> Option<QueuedTrack> {
        if self.is_looping {
            if let Some(current) = &self.current {
                return Some(current.clone());
            }
        }
        self.current = self.tracks.pop_front();
        self.current.clone()
    }

    pub fn clear(&mut self) {
        self.tracks.clear();
        self.current = None;
    }

    pub fn remove(&mut self, index: usize) -> Option<QueuedTrack> {
        self.tracks.remove(index)
    }

    pub fn len(&self) -> usize {
        self.tracks.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tracks.is_empty()
    }

    pub fn shuffle(&mut self) {
        use std::collections::VecDeque;
        let mut vec: Vec<_> = self.tracks.drain(..).collect();
        for i in (1..vec.len()).rev() {
            let j = rand_index(i + 1);
            vec.swap(i, j);
        }

        self.tracks = VecDeque::from(vec);
    }
}

fn rand_index(max: usize) -> usize {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .subsec_nanos() as usize;
    nanos % max
}
