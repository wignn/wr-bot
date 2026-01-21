use crate::services::music::player::{get_global_http, get_global_player};
use crate::services::music::queue::QueuedTrack;
use crate::utils::embed;
use lavalink_rs::client::LavalinkClient;
use lavalink_rs::model::events::{TrackEnd, TrackEndReason};
use serenity::all::{CreateEmbed, CreateEmbedFooter, CreateMessage, GuildId};

pub async fn handle_track_end(_client: LavalinkClient, event: &TrackEnd) {
    let should_continue: bool = event.reason.clone().into();
    let guild_id = GuildId::new(event.guild_id.0);
    let is_finished = matches!(event.reason, TrackEndReason::Finished);

    println!(
        "[MUSIC] Track ended in guild {}: {:?} - is_finished: {}, should_continue: {}",
        guild_id.get(),
        event.reason,
        is_finished,
        should_continue
    );

    if !is_finished {
        println!("[MUSIC] Track was stopped/replaced/failed, not advancing queue");
        return;
    }

    let player = match get_global_player() {
        Some(p) => p,
        None => {
            eprintln!("[MUSIC] Global music player not initialized");
            return;
        }
    };

    let player_ctx = match player.get_player_context(guild_id) {
        Some(ctx) => ctx,
        None => {
            println!(
                "[MUSIC] No player context found for guild {}",
                guild_id.get()
            );
            return;
        }
    };

    match player_ctx.get_player().await {
        Ok(p) if p.state.connected => {}
        _ => {
            println!("[MUSIC] Player disconnected, skipping track_end processing");
            player.set_current(guild_id, None);
            return;
        }
    }

    let text_channel = player.get_text_channel(guild_id);
    let queue = player.get_queue(guild_id);
    let volume = queue.volume;
    let is_looping = player.is_looping(guild_id);
    let (next_track, is_same_track) = player.next_track_with_loop_info(guild_id);

    match next_track {
        Some(track) => {
            println!("[MUSIC] Playing next track: {}", track.track.info.title);
            player.set_current(guild_id, Some(track.clone()));
            player.set_last_track_title(guild_id, Some(track.track.info.title.clone()));

            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

            if let Err(e) = player_ctx.play(&track.track).await {
                eprintln!("[MUSIC] Failed to play next track: {}", e);
                player.set_current(guild_id, None);
            } else {
                // Reset idle timer since we're playing music
                player.touch_activity(guild_id);

                println!(
                    "[MUSIC] Successfully started next track, is_same_track: {}",
                    is_same_track
                );

                // Only send Now Playing embed if this is a NEW track (not looping same track)
                if !is_same_track {
                    if let Some(channel_id) = text_channel {
                        if let Some(http) = get_global_http() {
                            let track_info = &track.track.info;
                            let duration_ms = track_info.length;
                            let duration = format!(
                                "{}:{:02}",
                                duration_ms / 60000,
                                (duration_ms % 60000) / 1000
                            );

                            let now_playing_embed = embed::now_playing(
                                &track_info.title,
                                &track_info.uri.clone().unwrap_or_default(),
                                &track_info.author,
                                &duration,
                                &track.requester_name,
                                volume,
                                is_looping,
                                track_info.artwork_url.as_deref(),
                            );

                            let message = CreateMessage::new().embed(now_playing_embed);
                            if let Err(e) = channel_id.send_message(http.as_ref(), message).await {
                                eprintln!("[MUSIC] Failed to send Now Playing embed: {}", e);
                            }
                        }
                    }
                } else {
                    println!("[MUSIC] Skipping Now Playing embed (track is looping)");
                }
            }
        }
        None => {
            println!("[MUSIC] Queue is empty, checking autoplay...");
            handle_autoplay(player, &player_ctx, guild_id, text_channel).await;
        }
    }
}

async fn handle_autoplay(
    player: &crate::services::music::MusicPlayer,
    player_ctx: &lavalink_rs::player_context::PlayerContext,
    guild_id: GuildId,
    text_channel: Option<serenity::all::ChannelId>,
) {
    use crate::services::youtube::get_global_youtube;

    if !player.is_autoplay(guild_id) {
        println!("[MUSIC] Autoplay is disabled");
        player.set_current(guild_id, None);
        return;
    }

    println!("[MUSIC] Autoplay is enabled, searching for related song...");

    let last_title = match player.get_last_track_title(guild_id) {
        Some(t) => t,
        None => {
            println!("[MUSIC] Autoplay: no last track to base search on");
            return;
        }
    };

    let video_id = player.get_last_video_id(guild_id);
    let played_ids = player.get_played_video_ids(guild_id);

    let lavalink_guild_id = lavalink_rs::model::GuildId(guild_id.get());

    let tracks: Vec<lavalink_rs::model::track::TrackData> = if let Some(ref vid) = video_id {
        // Use the current video ID to get its mix, this gives related songs
        let mix_url = format!("https://www.youtube.com/watch?v={}&list=RD{}", vid, vid);
        println!("[MUSIC] Autoplay loading YouTube Mix: {}", mix_url);

        match player
            .lavalink
            .load_tracks(lavalink_guild_id, &mix_url)
            .await
        {
            Ok(loaded) => {
                use lavalink_rs::model::track::TrackLoadData;
                match loaded.data {
                    Some(TrackLoadData::Playlist(p)) if p.tracks.len() > 1 => {
                        // Filter out already played tracks
                        p.tracks
                            .into_iter()
                            .skip(1) // Skip current track
                            .filter(|t| {
                                if let Some(ref uri) = t.info.uri {
                                    if let Some(track_vid) = extract_video_id(uri) {
                                        return !played_ids.contains(&track_vid);
                                    }
                                }
                                true
                            })
                            .collect()
                    }
                    Some(TrackLoadData::Track(t)) => vec![t],
                    Some(TrackLoadData::Search(t)) => t,
                    _ => vec![],
                }
            }
            Err(e) => {
                println!("[MUSIC] YouTube Mix failed: {}, falling back to search", e);
                vec![]
            }
        }
    } else {
        vec![]
    };

    let tracks = if tracks.is_empty() {
        println!("[MUSIC] Falling back to YouTube API search");
        let youtube = match get_global_youtube() {
            Some(yt) => yt,
            None => {
                println!("[MUSIC] Autoplay: YouTube API not available");
                player.set_current(guild_id, None);
                return;
            }
        };

        let simplified: String = last_title
            .split_whitespace()
            .take(2)
            .collect::<Vec<_>>()
            .join(" ");

        let search_query = format!("{} mix", simplified);
        println!("[MUSIC] Autoplay searching: {}", search_query);

        match youtube.search(&search_query, 5).await {
            Ok(videos) if !videos.is_empty() => {
                let video = if videos.len() > 1 {
                    &videos[1]
                } else {
                    &videos[0]
                };
                match player
                    .lavalink
                    .load_tracks(lavalink_guild_id, &video.url)
                    .await
                {
                    Ok(loaded) => {
                        use lavalink_rs::model::track::TrackLoadData;
                        match loaded.data {
                            Some(TrackLoadData::Track(t)) => vec![t],
                            Some(TrackLoadData::Search(t)) => t,
                            Some(TrackLoadData::Playlist(p)) => p.tracks,
                            _ => vec![],
                        }
                    }
                    Err(_) => vec![],
                }
            }
            _ => vec![],
        }
    } else {
        tracks
    };

    if tracks.is_empty() {
        println!("[MUSIC] Autoplay: no tracks found (all filtered or mix empty)");
        player.set_current(guild_id, None);
        // Clear history when we run out of tracks to allow fresh start
        player.clear_played_video_ids(guild_id);
        return;
    };

    // Pick first unplayed track instead of random to ensure variety
    let track = &tracks[0];
    println!(
        "[MUSIC] Autoplay found: {} (from {} candidates)",
        track.info.title,
        tracks.len()
    );

    // Extract and save video ID for next autoplay iteration
    if let Some(ref uri) = track.info.uri {
        if let Some(vid) = extract_video_id(uri) {
            // Add to played history
            player.add_played_video_id(guild_id, vid.clone());
            // Update last video ID so next mix is based on THIS song
            player.set_last_video_id(guild_id, Some(vid));
        }
    }

    player.set_last_track_title(guild_id, Some(track.info.title.clone()));

    let queued = QueuedTrack {
        track: track.clone(),
        requester_id: 0,
        requester_name: "Autoplay".to_string(),
    };

    player.set_current(guild_id, Some(queued));

    if let Err(e) = player_ctx.play(&track).await {
        eprintln!("[MUSIC] Autoplay failed to play: {}", e);
        player.set_current(guild_id, None);
    } else {
        // Reset idle timer since we're playing music
        player.touch_activity(guild_id);

        if let Some(channel_id) = text_channel {
            if let Some(http) = get_global_http() {
                let mut embed = CreateEmbed::new()
                    .title("Autoplay")
                    .description(format!(
                        "**[{}]({})**\nby {}",
                        track.info.title,
                        track.info.uri.clone().unwrap_or_default(),
                        track.info.author
                    ))
                    .color(0x1DB954)
                    .footer(CreateEmbedFooter::new("Use /autoplay to disable"));

                // Add thumbnail if available
                if let Some(ref artwork) = track.info.artwork_url {
                    embed = embed.thumbnail(artwork);
                }

                let message = CreateMessage::new().embed(embed);
                let _ = channel_id.send_message(http.as_ref(), message).await;
            }
        }
    }
}

/// Extract YouTube video ID from URL
fn extract_video_id(url: &str) -> Option<String> {
    // Handle youtu.be/ID format
    if url.contains("youtu.be/") {
        return url
            .split("youtu.be/")
            .nth(1)?
            .split('?')
            .next()
            .map(|s| s.to_string());
    }

    // Handle youtube.com?v=ID format
    if url.contains("youtube.com") {
        if let Some(v_param) = url.split("v=").nth(1) {
            return Some(v_param.split('&').next()?.to_string());
        }
    }

    None
}
