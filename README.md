# ============================================================
# VOCAI: Vocab+AI — README
# ============================================================

<div align="center">

![Vocai Logo](logo-transparent.svg)

# Vocai: Vocab+AI

**AI-powered vocabulary learning with spaced repetition**

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.94+-blue.svg)](https://www.rust-lang.org)
[![W9 Labs](https://img.shields.io/badge/W9-Labs-6366F1)](https://w9.nu)

</div>

---

## 🎯 Features

- **🤖 AI Flashcard Generation** — Powered by NVIDIA AI, generate vocabulary flashcards by topic
- **🧠 Spaced Repetition System (SRS)** — Hybrid SM-2 + Leitner algorithm for optimal memory retention
- **🏝️ Vocabulary Islands** — Learn words in contextual topics (cooking, politics, tech, etc.)
- **📊 Smart Analytics** — Track your learning progress, streaks, and mastered words
- **🔐 OAuth Authentication** — Secure login via W9 Labs unified auth system
- **🎨 Modern UI** — 2026 trending color palette with 8-bit voxel arcade elements

## 🛠️ Tech Stack

- **Backend:** Rust 1.94 + Axum 0.7 + tokio
- **Database:** PostgreSQL 16 (shared with W9 Labs ecosystem)
- **AI:** NVIDIA API (free tier, extensible to other providers)
- **Authentication:** OAuth 2.0 via w9-db
- **Deployment:** Docker + Docker Compose + Caddy reverse proxy
- **Frontend:** Server-rendered HTML + vanilla JavaScript

## 🚀 Quick Start

### Local Development

```bash
# Clone the repository
git clone https://github.com/w9labs/vocai.git
cd vocai

# Copy environment file
cp .env.example .env

# Start PostgreSQL (or use existing w9-postgres)
# Create database: CREATE DATABASE w9_vocabai;

# Run the server
cd server
cargo run
```

### Docker Compose

```bash
docker compose up -d
```

## 📁 Project Structure

```
vocai/
├── server/
│   ├── Cargo.toml          # Rust dependencies
│   └── src/
│       ├── main.rs         # Application entry point
│       ├── db.rs           # Database connection & migrations
│       ├── models.rs       # Data models
│       ├── handlers/       # Route handlers
│       │   ├── mod.rs
│       │   ├── auth.rs     # OAuth authentication
│       │   ├── flashcards.rs # Flashcard CRUD & generation
│       │   ├── islands.rs  # Vocabulary islands
│       │   └── review.rs   # SRS review sessions
│       ├── nvidia.rs       # NVIDIA AI integration
│       ├── srs.rs          # Spaced repetition algorithms
│       ├── auth.rs         # OAuth callback & login
│       └── session.rs      # Session management
├── public/
│   └── assets/
│       ├── css/vocai.css   # 2026 color palette + voxel theme
│       └── js/vocai.js     # Frontend interactions
├── Dockerfile              # Multi-stage Docker build
├── docker-compose.yml      # Docker Compose config
└── .env.example            # Environment template
```

## 🧠 Spaced Repetition Algorithm

Vocai uses a **hybrid SRS** combining:

1. **SM-2 Algorithm** (SuperMemo) — Calculates optimal review intervals based on recall quality
2. **Leitner System** — Moves mastered words to less frequent review boxes

### Review Quality Scale

| Rating | Meaning | Interval Effect |
|--------|---------|-----------------|
| 0 | Complete blackout | Reset to 1m |
| 1 | Incorrect response | Reset to 1m |
| 2 | Correct recalled, hard | Short interval |
| 3 | Correct recalled, difficult | Normal interval |
| 4 | Perfect response | Extended interval |
| 5 | Perfect + trivial | Maximum interval |

### Leitner Boxes

| Box | Review Interval | Description |
|-----|----------------|-------------|
| 1 | Every day | New/difficult words |
| 2 | Every 3 days | Learning words |
| 3 | Every 7 days | Familiar words |
| 4 | Every 14 days | Well-known words |
| 5 | Every 30 days | Mastered words |

## 🌐 Deployment

### VPS (W9 Labs Server)

Vocai deploys to the W9 Labs VPS alongside other services:

```bash
# SSH to server
ssh -p 22001 root@ffm.w9.nu

# Navigate to deployment directory
cd /opt/w9-labs

# Pull latest and redeploy
docker compose pull vocai && docker compose up -d vocai
```

### Caddy Configuration

Add to `/etc/caddy/Caddyfile`:

```caddy
vocai.top {
    reverse_proxy vocai:3010
}
```

### Cloudflare Pages (Alternative)

For static frontend:

```bash
wrangler pages deploy public --project-name=vocai
```

## 📊 Database Schema

### Tables

- `users` — User accounts (synced with w9-db OAuth)
- `vocabulary_islands` — Topic-based word collections
- `flashcards` — Individual vocabulary cards
- `srs_reviews` — Spaced repetition progress
- `study_sessions` — Review history
- `user_stats` — Aggregated learning statistics

## 🔐 OAuth Integration

Vocai uses the W9 Labs unified OAuth 2.0 system:

1. User clicks "Login with W9" on `vocai.top`
2. Redirect to `db.w9.nu/oauth/authorize`
3. User authenticates via w9-db
4. Callback to `vocai.top/auth/callback` with auth code
5. Exchange code for token via `/oauth/token`
6. Session created, user redirected to dashboard

## 🎨 Design System

### 2026 Trending Color Palette

- **Primary:** Electric Indigo (#6366F1)
- **Secondary:** Neo Mint (#A7F3D0)
- **Accent:** Digital Lavender (#E9D5FF)
- **Background:** Deep Space (#0F172A)
- **Success:** Cyber Green (#22D3EE)
- **Warning:** Solar Orange (#FB923C)
- **Error:** Crimson Glitch (#F43F5E)

### Typography

- **Headings:** Space Grotesk (geometric, modern)
- **Body:** Inter (clean, readable)
- **Code/Accent:** JetBrains Mono (technical)

## 📝 License

MIT License — see [LICENSE](LICENSE) for details.

## 🤝 Contributing

Contributions welcome! Please read our [Contributing Guide](CONTRIBUTING.md) first.

## 💜 Built with love by W9 Labs

Part of the W9 Network ecosystem. Learn more at [w9.nu](https://w9.nu)
