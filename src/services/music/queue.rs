use lavalink_rs::model::track::TrackData;
use serenity::all::ChannelId;
use std::collections::VecDeque;
use std::time::Instant;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoopMode {
    Off,
    Track, // Repeat current track
    Queue, // Repeat entire queue
}

impl Default for LoopMode {
    fn default() -> Self {
        Self::Off
    }
}

#[derive(Debug, Clone)]
pub struct MusicQueue {
    pub tracks: VecDeque<QueuedTrack>,
    pub played_tracks: VecDeque<QueuedTrack>,
    pub current: Option<QueuedTrack>,
    pub volume: u8,
    pub loop_mode: LoopMode,
    pub is_looping: bool,
    pub is_paused: bool,
    pub is_autoplay: bool,
    pub last_track_title: Option<String>,
    pub last_video_id: Option<String>,
    pub played_video_ids: VecDeque<String>,
    pub text_channel_id: Option<ChannelId>,
    pub last_activity: Instant, // Track when music was last active
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
            played_tracks: VecDeque::new(),
            current: None,
            volume: 100,
            loop_mode: LoopMode::Off,
            is_looping: false,
            is_paused: false,
            is_autoplay: false,
            last_track_title: None,
            last_video_id: None,
            played_video_ids: VecDeque::with_capacity(20),
            text_channel_id: None,
            last_activity: Instant::now(),
        }
    }

    /// Update last activity timestamp
    pub fn touch_activity(&mut self) {
        self.last_activity = Instant::now();
    }

    /// Check if queue has been idle for the given duration
    pub fn is_idle_for(&self, duration: std::time::Duration) -> bool {
        self.current.is_none() && self.last_activity.elapsed() >= duration
    }

    pub fn add(&mut self, track: QueuedTrack) {
        self.tracks.push_back(track);
    }

    pub fn next_with_loop_info(&mut self) -> (Option<QueuedTrack>, bool) {
        if self.loop_mode == LoopMode::Track || self.is_looping {
            if let Some(current) = &self.current {
                return (Some(current.clone()), true);
            }
        }

        if let Some(current) = self.current.take() {
            if self.loop_mode == LoopMode::Queue {
                self.played_tracks.push_back(current);
            }
        }

        if let Some(next) = self.tracks.pop_front() {
            self.current = Some(next.clone());
            return (Some(next), false);
        }

        if self.loop_mode == LoopMode::Queue && !self.played_tracks.is_empty() {
            self.tracks = std::mem::take(&mut self.played_tracks);
            if let Some(next) = self.tracks.pop_front() {
                self.current = Some(next.clone());
                return (Some(next), false);
            }
        }

        self.current = None;
        (None, false)
    }

    pub fn next(&mut self) -> Option<QueuedTrack> {
        self.next_with_loop_info().0
    }

    pub fn clear(&mut self) {
        self.tracks.clear();
        self.played_tracks.clear();
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
