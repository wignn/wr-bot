# Worm

A high-performance Discord bot built with Rust, featuring music playback, AI chat, moderation tools, and game redemption code tracking.

## Features

### Music
- Lavalink-powered audio streaming with support for YouTube, Spotify, SoundCloud, and more
- Queue management with shuffle, loop, and skip controls
- Autoplay mode that automatically queues related tracks
- Real-time now playing embeds with progress tracking

### AI Chat
- Conversational AI powered by OpenRouter API
- Configurable model selection
- Context-aware responses with custom system prompts

### Moderation
- Warn, kick, ban, and timeout commands
- Warning history tracking per user
- Moderation logs with detailed audit trails
- Permission-based access control

### Game Redemption Codes
- Automatic scraping for Genshin Impact, Wuthering Waves, Honkai Star Rail, and Zenless Zone Zero
- Per-channel notification setup
- Code history with status tracking

## Requirements

- Rust 1.70+
- Docker & Docker Compose (recommended)
- Lavalink 4.x server
- SQLite (bundled)

## Quick Start

### Using Docker (Recommended)

```bash
# Clone the repository
git clone https://github.com/wignn/discord-bot
cd wr-bot

# Copy and configure environment
cp .env.example .env
# Edit .env with your tokens

# Start services
docker compose up -d
```

### Manual Setup

```bash
# Build
cargo build --release

# Run Lavalink separately (see lavalink/application.yml)
java -jar Lavalink.jar

# Start the bot
./target/release/worm
```

## Configuration

Create a `.env` file in the project root:

```env
# Discord
TOKEN=your_discord_bot_token
CLIENT_ID=your_client_id

# AI (Optional)
API_KEY=your_openrouter_api_key
BASE_URL=https://openrouter.ai/api/v1
MODEL_AI=your_preferred_model

# Lavalink
LAVALINK_HOST=localhost
LAVALINK_PORT=2333
LAVALINK_PASSWORD=youshallnotpass

# YouTube (for search)
YOUTUBE_API_KEY=your_youtube_api_key
```

## Commands

### Music
| Command | Description |
|---------|-------------|
| `/play <query>` | Play a song or add to queue |
| `/skip` | Skip current track |
| `/queue` | Show current queue |
| `/pause` | Pause playback |
| `/resume` | Resume playback |
| `/stop` | Stop and clear queue |
| `/nowplaying` | Show current track |
| `/volume <0-150>` | Adjust volume |
| `/loop` | Toggle loop mode |
| `/shuffle` | Shuffle the queue |
| `/autoplay` | Toggle autoplay mode |

### Moderation
| Command | Description |
|---------|-------------|
| `/warn <user> <reason>` | Issue a warning |
| `/kick <user> [reason]` | Kick a member |
| `/ban <user> [reason]` | Ban a member |
| `/timeout <user> <duration>` | Timeout a member |
| `/warnings <user>` | View warning history |

### Utility
| Command | Description |
|---------|-------------|
| `/worm <message>` | Chat with AI |
| `/ping` | Check bot latency |
| `/sysinfo` | System information |
| `/redeem_setup` | Configure code notifications |
| `/redeem_codes <game>` | View available codes |

## Project Structure

```
src/
├── commands/       # Command implementations
├── handlers/       # Event handlers
├── repository/     # Database operations
├── scraper/        # Code scraping logic
├── services/       # Core services (AI, music, etc.)
└── utils/          # Shared utilities
```

## Tech Stack

- **Framework**: [Serenity](https://github.com/serenity-rs/serenity) + [Poise](https://github.com/serenity-rs/poise)
- **Audio**: [Lavalink](https://github.com/lavalink-devs/Lavalink) via lavalink-rs
- **Database**: SQLite with rusqlite
- **Runtime**: Tokio