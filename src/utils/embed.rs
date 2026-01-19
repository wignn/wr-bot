use poise::serenity_prelude::CreateEmbed;

pub const COLOR_SUCCESS: u32 = 0x2ECC71; // Green
pub const COLOR_ERROR: u32 = 0xE74C3C; // Red
pub const COLOR_WARNING: u32 = 0xF39C12; // Orange
pub const COLOR_INFO: u32 = 0x3498DB; // Blue
pub const COLOR_MUSIC: u32 = 0x1DB954; // Spotify Green

pub fn success(title: &str, description: &str) -> CreateEmbed {
    CreateEmbed::new()
        .title(format!("[OK] {}", title))
        .description(description)
        .color(COLOR_SUCCESS)
}

pub fn error(title: &str, description: &str) -> CreateEmbed {
    CreateEmbed::new()
        .title(format!("[ERROR] {}", title))
        .description(description)
        .color(COLOR_ERROR)
}

pub fn warning(title: &str, description: &str) -> CreateEmbed {
    CreateEmbed::new()
        .title(format!("[WARN] {}", title))
        .description(description)
        .color(COLOR_WARNING)
}

pub fn info(title: &str, description: &str) -> CreateEmbed {
    CreateEmbed::new()
        .title(title)
        .description(description)
        .color(COLOR_INFO)
}

pub fn music(title: &str, description: &str) -> CreateEmbed {
    CreateEmbed::new()
        .title(title)
        .description(description)
        .color(COLOR_MUSIC)
}

pub fn now_playing(
    title: &str,
    url: &str,
    author: &str,
    duration: &str,
    requester: &str,
    volume: u8,
    is_looping: bool,
    artwork_url: Option<&str>,
) -> CreateEmbed {
    let mut embed = CreateEmbed::new()
        .title("🎵 Now Playing")
        .description(format!("**[{}]({})**", title, url))
        .field("Artist", author, true)
        .field("Duration", duration, true)
        .field("Requested by", requester, true)
        .field("Volume", format!("{}%", volume), true)
        .field("Loop", if is_looping { "On" } else { "Off" }, true)
        .color(COLOR_MUSIC);

    if let Some(art) = artwork_url {
        if !art.is_empty() {
            embed = embed.thumbnail(art);
        }
    }

    embed
}

pub fn added_to_queue(
    title: &str,
    url: &str,
    duration: &str,
    position: usize,
    requester: &str,
    artwork_url: Option<&str>,
) -> CreateEmbed {
    let mut embed = CreateEmbed::new()
        .title("Added to Queue")
        .description(format!("**[{}]({})**", title, url))
        .field("Duration", duration, true)
        .field("Position", format!("#{}", position), true)
        .field("Requested by", requester, true)
        .color(COLOR_MUSIC);

    if let Some(art) = artwork_url {
        if !art.is_empty() {
            embed = embed.thumbnail(art);
        }
    }

    embed
}

pub fn playlist_added(
    first_track_title: &str,
    first_track_url: &str,
    track_count: usize,
    requester: &str,
    artwork_url: Option<&str>,
) -> CreateEmbed {
    let mut embed = CreateEmbed::new()
        .title("🎶 Playlist Added")
        .description(format!(
            "**[{}]({})** and **{} more tracks** added to queue",
            first_track_title,
            first_track_url,
            track_count.saturating_sub(1)
        ))
        .field("Total Tracks", format!("{}", track_count), true)
        .field("Requested by", requester, true)
        .color(COLOR_MUSIC);

    if let Some(art) = artwork_url {
        if !art.is_empty() {
            embed = embed.thumbnail(art);
        }
    }

    embed
}


pub const COLOR_JOIN: u32 = 0x43B581; // Green for joins
pub const COLOR_LEAVE: u32 = 0xF04747; // Red for leaves

pub fn member_join(
    username: &str,
    user_id: u64,
    account_created: &str,
    member_count: u64,
    avatar_url: Option<&str>,
) -> CreateEmbed {
    let mut embed = CreateEmbed::new()
        .title("📥 Member Joined")
        .description(format!(
            "**User:** {} (`{}`)\n**Account Created:** {}\n**Member Count:** {}",
            username, user_id, account_created, member_count
        ))
        .color(COLOR_JOIN);

    if let Some(avatar) = avatar_url {
        embed = embed.thumbnail(avatar);
    }

    embed
}

pub fn member_leave(
    username: &str,
    user_id: u64,
    joined_at: Option<&str>,
    member_count: u64,
    avatar_url: Option<&str>,
) -> CreateEmbed {
    let joined_info = joined_at
        .map(|j| format!("\n**Joined:** {}", j))
        .unwrap_or_default();

    let mut embed = CreateEmbed::new()
        .title("📤 Member Left")
        .description(format!(
            "**User:** {} (`{}`){}\\n**Member Count:** {}",
            username, user_id, joined_info, member_count
        ))
        .color(COLOR_LEAVE);

    if let Some(avatar) = avatar_url {
        embed = embed.thumbnail(avatar);
    }

    embed
}


pub fn voice_join(
    username: &str,
    _user_id: u64,
    channel_name: &str,
    avatar_url: Option<&str>,
) -> CreateEmbed {
    let mut embed = CreateEmbed::new()
        .title("Joined Voice Channel")
        .description(format!("**{}** joined **{}**", username, channel_name))
        .color(COLOR_JOIN);

    if let Some(avatar) = avatar_url {
        embed = embed.thumbnail(avatar);
    }

    embed
}

pub fn voice_leave(
    username: &str,
    _user_id: u64,
    channel_name: &str,
    avatar_url: Option<&str>,
) -> CreateEmbed {
    let mut embed = CreateEmbed::new()
        .title("🔇 Left Voice Channel")
        .description(format!("**{}** left **{}**", username, channel_name))
        .color(COLOR_LEAVE);

    if let Some(avatar) = avatar_url {
        embed = embed.thumbnail(avatar);
    }

    embed
}
