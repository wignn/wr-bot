use std::path::PathBuf;
use std::sync::OnceLock;
use tokio::sync::Mutex;
use uuid::Uuid;
use yt_dlp::model::selector::{AudioQuality, VideoQuality};
use yt_dlp::Youtube;

static GLOBAL_DOWNLOADER: OnceLock<Mutex<Option<Youtube>>> = OnceLock::new();

#[derive(Debug, Clone, PartialEq)]
pub enum Platform {
    YouTubeShorts,
    Instagram,
    Facebook,
    TikTok,
    Unknown,
}

impl Platform {
    pub fn from_url(url: &str) -> Platform {
        let url = url.to_lowercase();

        if url.contains("youtube.com/shorts") || (url.contains("youtu.be") && url.contains("/shorts")){
            return Platform::YouTubeShorts;
        }

        if url.contains("instagram.com/reel") || url.contains("instagram.com/reels") {
            return Platform::Instagram;
        }

        if url.contains("facebook.com/reel") || url.contains("fb.watch") {
            return Platform::Facebook;
        }
        if url.contains("tiktok.com") || url.contains("vm.tiktok") { 
            return Platform::TikTok 
        }
        Platform::Unknown
    }

    pub fn name(&self) -> &str {
        match self {
            Platform::YouTubeShorts => "YouTube Shorts",
            Platform::Instagram => "Instagram",
            Platform::Facebook => "Facebook",
            Platform::TikTok => "TikTok",
            Platform::Unknown => "Unknown",
        }
    }

    pub fn is_supported(&self) -> bool {
        !matches!(self, Platform::Unknown)
    }
}

pub struct Downloader;

impl Downloader {
    async fn get_or_init_yt() -> Result<Youtube, Box<dyn std::error::Error + Send + Sync>> {
        let lock = GLOBAL_DOWNLOADER.get_or_init(|| Mutex::new(None));
        let mut guard = lock.lock().await;

        if guard.is_none() {
            let executables_dir = PathBuf::from("bin");
            let output_dir = PathBuf::from("output");

            if !output_dir.exists() {
                tokio::fs::create_dir_all(&output_dir).await?;
            }

            println!("[VIDEO] Initializing yt-dlp binaries...");
            let yt = Youtube::with_new_binaries(executables_dir, output_dir).await?;
            *guard = Some(yt);
            println!("[VIDEO] yt-dlp initialized successfully");
        }

        Ok(guard.as_ref().unwrap().clone())
    }

    pub fn detect_platform(url: &str) -> Platform {
        Platform::from_url(url)
    }

    pub async fn download(url: &str) -> Result<PathBuf, Box<dyn std::error::Error + Send + Sync>> {
        let platform = Platform::from_url(url);

        if !platform.is_supported() {
            return Err("Platform tidak didukung. Gunakan link dari YouTube, Instagram, Facebook, atau TikTok.".into());
        }

        let yt = Self::get_or_init_yt().await?;

        let id = Uuid::new_v4();
        let filename = format!("{}.mp4", id);

       let path = yt
            .download(url.to_string(), &filename)
            .video_quality(VideoQuality::Medium) // 720p
            .audio_quality(AudioQuality::Medium) // 128kbps
            .execute()
            .await?;

        Ok(path)
    }

    pub async fn delete_video(path: &PathBuf) -> Result<(), std::io::Error> {
        if path.exists() {
            tokio::fs::remove_file(path).await?;
        }
        Ok(())
    }
}
