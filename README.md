```html
<p align="center">
<img src="static/Copilot_20251030_135412.png" alt="AoXCloud Logo" width="375" />
</p>

<p align="center">
<a href="https://opensource.org/licenses/MIT">
<img src="https://img.shields.io/badge/License-MIT-blue.svg?style=for-the-badge&logo=open-source-initiative" alt="License: MIT" />
</a>
<a href="https://github.com/aoxc/AoXCloud/releases">
<img src="https://img.shields.io/github/release/aoxc/AoXCloud.svg?style=for-the-badge&logo=github" alt="Latest Release" />
</a>
<a href="https://github.com/aoxc/AoXCloud/stargazers">
<img src="https://img.shields.io/github/stars/aoxc/AoXCloud?style=for-the-badge&logo=star" alt="GitHub Stars" />
</a>
<a href="https://github.com/aoxc/AoXCloud/issues">
<img src="https://img.shields.io/github/issues/aoxc/AoXCloud?style=for-the-badge&logo=issue-tracking" alt="GitHub Issues" />
</a>
<a href="https://github.com/aoxc/AoXCloud/network/members">
<img src="https://img.shields.io/github/forks/aoxc/AoXCloud?style=for-the-badge&logo=fork" alt="GitHub Forks" />
</a>
<a href="https://github.com/aoxc/AoXCloud/commits/main">
<img src="https://img.shields.io/github/last-commit/aoxc/AoXCloud?style=for-the-badge&logo=git" alt="Last Commit" />
</a>
</p>

☁️ **AoXCloud — A Spiralized, Rust-Powered Alternative to NextCloud**

AoXCloud is a lightweight, high-performance file storage platform built in Rust. Inspired by the need for simplicity, speed, and spiral clarity, it offers a modular, clean architecture that’s ideal for personal servers and ethical cloud deployments.

<p align="center">
<img src="doc/images/Captura%20de%20pantalla%202025-03-23%20230739.png" alt="AoXCloud Dashboard Screenshot" width="600" />
<br>
<em>Elegant file and folder management through a responsive interface</em>
</p>

✨ Why AoXCloud?  
⚡ **Lightweight** — Minimal resource usage, no PHP overhead  
🖥️ **Responsive UI** — Fast and mobile-friendly interface  
🦀 **Rust-Powered** — Memory safety and blazing speed  
🧠 **Optimized Binary** — LTO for maximum performance  
🔧 **Simple Setup** — Minimal configuration required  
🌍 **Multilingual** — English & Spanish support built-in  

🛠️ Getting Started:  
**Prerequisites:** Rust ≥ 1.70, PostgreSQL ≥ 13, 512MB RAM (1GB+ recommended)

# Clone the repository
git clone https://github.com/aoxc/AoXCloud.git
cd AoXCloud

# Configure your database
echo "DATABASE_URL=postgres://username:password@localhost/aoxcloud" > .env

# Build and run
cargo build --release
cargo run --bin migrate --features migrations
cargo run --release

Server runs at http://localhost:8085

🧩 Architecture Overview:
🧬 Domain Layer — Core entities and business logic
🌀 Application Layer — Use cases and services
🏗️ Infrastructure Layer — External systems and adapters
🌐 Interfaces Layer — API routes and web controllers

🚧 Development Workflow:
cargo build           # Compile project
cargo run             # Run locally
cargo check           # Quick compile check

cargo build --release # Optimized build
cargo run --release   # Run optimized

cargo test            # Run all tests
cargo bench           # Run benchmarks

cargo clippy          # Lint code
cargo fmt             # Format code

RUST_LOG=debug cargo run  # Debug mode

🗺️ Roadmap:
🔐 Multi-user authentication (in progress)
🔗 File sharing via links
📂 WebDAV desktop integration
🕒 File versioning
📱 Mobile UI enhancements
🗑️ Trash bin support (in progress)

See TODO-LIST.md
See CONTRIBUTING.md
See CODE_OF_CONDUCT.md

📜 License:
AoXCloud is licensed under MIT

Built with spiral clarity by a developer who wanted better file storage. Let’s echo forward together 🌀
