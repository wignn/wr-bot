use std::path::PathBuf;
use uuid::Uuid;
use yt_dlp::Youtube;

#[derive(Debug, Clone, PartialEq)]
pub enum Platform {
    YouTube,
    YouTubeShorts,
    Instagram,
    Facebook,
    TikTok,
    Unknown,
}

impl Platform {
    pub fn from_url(url: &str) -> Self {
        let url_lower = url.to_lowercase();
        
        if url_lower.contains("youtube.com/shorts") || url_lower.contains("youtu.be") && url_lower.contains("/shorts") {
            Platform::YouTubeShorts
        } else if url_lower.contains("youtube.com") || url_lower.contains("youtu.be") {
            Platform::YouTube
        } else if url_lower.contains("instagram.com") || url_lower.contains("instagr.am") {
            Platform::Instagram
        } else if url_lower.contains("facebook.com") || url_lower.contains("fb.watch") || url_lower.contains("fb.com") {
            Platform::Facebook
        } else if url_lower.contains("tiktok.com") || url_lower.contains("vm.tiktok") {
            Platform::TikTok
        } else {
            Platform::Unknown
        }
    }

    pub fn name(&self) -> &str {
        match self {
            Platform::YouTube => "YouTube",
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

pub struct Downloader {
    yt: Youtube,
}

impl Downloader {
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let executables_dir = PathBuf::from("bin");
        let output_dir = PathBuf::from("output");

        if !output_dir.exists() {
            std::fs::create_dir_all(&output_dir)?;
        }

        let yt = Youtube::with_new_binaries(executables_dir, output_dir).await?;
        Ok(Self { yt })
    }

    pub fn detect_platform(url: &str) -> Platform {
        Platform::from_url(url)
    }

    pub async fn download(&self, url: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
        let platform = Platform::from_url(url);
        
        if !platform.is_supported() {
            return Err("Platform tidak didukung. Gunakan link dari YouTube, Instagram, Facebook, atau TikTok.".into());
        }

        let id = Uuid::new_v4();
        let filename = format!("{}.mp4", id);
        let path = self.yt.download(url.to_string(), &filename).execute().await?;
        Ok(path)
    }

    pub fn delete_video(path: &PathBuf) -> Result<(), std::io::Error> {
        if path.exists() {
            std::fs::remove_file(path)?;
        }
        Ok(())
    }
}
