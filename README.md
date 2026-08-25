<div align="center">

# 🎨 Auto Comfy Maker

**Turn tag libraries + themes into ComfyUI-ready prompts — automatically.**

[English](./README.md) · [简体中文](./README.zh.md)

</div>

<div align="center">

[![License: MIT OR Apache-2.0](https://img.shields.io/badge/License-MIT%20OR%20Apache--2.0-blue.svg)](./LICENSE)
[![Rust 2021](https://img.shields.io/badge/Rust-2021%20Edition-orange.svg)](https://www.rust-lang.org/)
[![ComfyUI](https://img.shields.io/badge/ComfyUI-Compatible-blueviolet.svg)](https://github.com/comfyanonymous/ComfyUI)
[![Tests](https://img.shields.io/badge/tests-70_passing-success.svg)](#-development)
[![Cross compile](https://img.shields.io/badge/Linux_x86__64-static-lightgrey.svg)](#-deployment)

</div>

---

## Why?

You have a **ComfyUI server** and a **bucket of tags** you want to combine into prompts.

Manually editing prompts is fun — once. Doing it a hundred times is not. Auto Comfy Maker

- pulls tags from per-dimension pools,
- composes them into a stable prompt (with conflicts, weights, optional LLM polish),
- injects the prompt into a ComfyUI workflow JSON,
- submits, polls, downloads — without you writing a line of glue code.

The 5-minute journey: write a theme → run `generate` → get a picture.

---

## ✨ Features

| | |
|---|---|
| 🏷️ **Tag-as-data** | Tags live as plain text files grouped by dimension. Edit them in any editor. |
| 🎨 **Themes** | One TOML file = one style. Declare `fixed / random / optional` slots + conflicts. |
| 🔗 **Composable** | Mix multiple themes, swap prompts mid-batch, no code changes. |
| 🤖 **LLM optional** | Refine tags with OpenAI / Anthropic (via [rig-core](https://github.com/0xPlaygrounds/rig)). Falls back gracefully. |
| 📡 **ComfyUI native** | Submits a workflow JSON, polls, downloads the PNG — same as ComfyUI's "Queue Prompt". |
| ⏰ **Four scheduler modes** | `interval` · `cron` · `at` · `task-interval` — first three are mutually exclusive; `task-interval` stands alone |
| 📦 **Single static binary** | Cross-compiles to a fully-static 5.3 MB Linux binary via `cargo-zigbuild`. |
| 🛡️ **Configuration layered** | `default.toml` (checked in) + `local.toml` (gitignored) + env vars. |

---

## 🚀 Quick Start

> [!NOTE]
> Assumes a running ComfyUI server at `http://127.0.0.1:8188` (default). Override via `config/local.toml` or `COMFYUI_URL` env var.

```bash
# 1. Build (release)
cargo build --release

# 2. Generate one image with the bundled Anima theme
./target/release/auto-comfy-maker generate \
  --theme anima-simple \
  --lang en

# 3. Or batch 10 random combinations
./target/release/auto-comfy-maker batch -n 10 --theme anima-drawing-v5

# 4. Or run a daemon that generates every hour
./target/release/auto-comfy-maker daemon --interval 1h --mode auto --theme anima-simple
```

First image typically appears in `output/<YYYY-MM-DD>/<timestamp>_<hash>.png`.

---

## 🎨 Themes at a Glance

Themes are **declarative recipes** — they tell the engine which tag dimensions to sample, how many from each, and what conflicts to avoid.

```toml
# themes/anima-simple.toml
[meta]
id = "anima-simple"
name = "Anima Simple"
lang = "en"

[order.fixed]
quality     = { file = "tags/en/quality.txt",     count = 1 }
artist      = { file = "tags/en/_simple/artist_fixed.txt", count = 1 }

[order.random]
character   = { file = "tags/en/character.txt",   count = 1 }
pose        = { file = "tags/en/pose.txt",        count = 1, max = 2 }
background  = { file = "tags/en/background.txt",  count = 1 }
```

Bundled themes:

| ID | Style | Lang |
|---|---|---|
| `anima-simple` | Minimal, 1girl + fixed artist | `en` |
| `anima-drawing-v5` | Full Danbooru, era-aware conflicts | `en` |
| `portrait` | 中文古风人物 | `zh` |
| `portrait-en` | English portrait variant | `en` |

> [!TIP]
> Want a new style? Drop a `<your-style>.toml` into `themes/`. No code changes. The README at `themes/README.md` has the schema.

---

## 🔌 Workflow Templates

A ComfyUI workflow is **JSON data**, not code. Drop your exported workflow into `templates/`, mark the injection points with `${positive_prompt}` / `${seed}` / `${width}` / `${height}` (or their Chinese aliases), done.

```json
{
  "68": {
    "inputs": { "text": "${positive_prompt}" },
    "class_type": "Text Multiline"
  },
  "28": {
    "inputs": { "width": "${width}", "height": "${height}" },
    "class_type": "EmptyLatentImage"
  }
}
```

> [!IMPORTANT]
> Every template is validated at load time — missing any of the four placeholders fails fast with a clear error message. No silent literal leaking into ComfyUI.

Bundled templates: `anima`, `anima-aesthetic`, `anima-lora`, `zimage`.

---

## 📦 CLI Cheatsheet

```text
auto-comfy-maker generate   # one image, one theme, manual control
auto-comfy-maker batch      # N images, randomized combos
auto-comfy-maker daemon     # interval / cron / at / task-interval
auto-comfy-maker tags       # list / show / add / remove tags
auto-comfy-maker config     # show merged config / validate
```

<details>
<summary><b>generate</b> — single image, exact control</summary>

```bash
auto-comfy-maker generate \
  --theme anima-simple \
  --template anima-lora \
  --lang en \
  --seed 42 \
  --width 768 --height 1536 \
  --no-send   # only print the prompt, skip ComfyUI
```

</details>

<details>
<summary><b>batch</b> — N random combos</summary>

```bash
auto-comfy-maker batch -n 20 --theme anima-simple --lang en
```

</details>

<details>
<summary><b>daemon</b> — four mutually exclusive modes</summary>

| Mode | Flag | Example |
|---|---|---|
| Interval | `--interval 30m` | every 30 minutes |
| Cron | `--cron "0 9 * * *"` | every day at 09:00 |
| At list | `--at 2026-09-01T09:00:00+08:00` | one-shot at a moment |
| Task interval | `--task-interval 5s` | continuous, 5s between jobs |

Daemon persists to `config/schedule.toml` (gitignored) — survives restarts, no double-fires.

</details>

Full reference: [`docs/cli.md`](./docs/cli.md).

---

## 🧱 Tech Stack

| Layer | Choice | Why |
|---|---|---|
| Language | Rust 2021 | Memory safety, zero-cost async |
| Runtime | Tokio 1.x | ComfyUI HTTP long-poll |
| LLM | rig-core 0.40 | Unified OpenAI / Anthropic abstraction |
| HTTP | reqwest | ComfyUI REST only |
| CLI | clap 4 (derive) | Subcommands + auto-generated `--help` |
| Config | serde + toml | Strongly-typed config |
| Errors | thiserror (lib) + anyhow (bin) | Clean separation by layer |
| Logging | tracing + tracing-subscriber | Structured, level-filtered |

> [!NOTE]
> No database. No message queue. No web framework. Just a binary that talks to ComfyUI and writes PNGs.

---

## 📦 Deployment

### macOS → Linux (static binary)

```bash
# One-time: install cross-compile toolchain
brew install zig
cargo install cargo-zigbuild
export PATH="$HOME/.cargo/bin:$PATH"

# Build (uses the proxy if you're behind one)
export https_proxy=http://127.0.0.1:7890 \
       http_proxy=http://127.0.0.1:7890 \
       all_proxy=socks5://127.0.0.1:7890

cargo zigbuild --release --target x86_64-unknown-linux-musl
# → target/x86_64-unknown-linux-musl/release/auto-comfy-maker (5.3 MB, statically linked)
```

### Run as a systemd service

```ini
# /etc/systemd/system/auto-comfy-maker.service
[Unit]
Description=Auto Comfy Maker
After=network-online.target

[Service]
Type=simple
User=jiaoma
WorkingDirectory=/home/jiaoma/auto-comfy-maker
ExecStart=/home/jiaoma/auto-comfy-maker/auto-comfy-maker \
  daemon --interval 1h --mode auto --theme anima-simple
Restart=on-failure
RestartSec=10

[Install]
WantedBy=multi-user.target
```

---

## 📚 Documentation

| | |
|---|---|
| 🏛 [Architecture & data flow](./docs/architecture.md) | How pieces fit together |
| 🎮 [CLI reference](./docs/cli.md) | Every flag, every default |
| 🧬 [Prompt engine](./docs/prompt-engine.md) | combine() → refine() pipeline |
| ⏰ [Scheduler](./docs/scheduler.md) | The four daemon modes in depth |
| 🧩 [Workflow templates](./docs/workflow-template.md) | Placeholder contract |
| ⚙️ [Config layering](./docs/config.md) | default / local / env precedence |
| 🤖 [AI collaboration guide (CLAUDE.md)](./CLAUDE.md) | Conventions for AI-assisted work |
| 🛣 [Roadmap](./docs/ROADMAP.md) | Where this is going |

---

## 🛠 Development

```bash
cargo build            # debug
cargo test             # 70 unit tests, all green
cargo clippy -- -D warnings
cargo fmt --check
```

Conventions live in [`docs/constraint/`](./docs/constraint/README.md): naming, structure, format, commits, testing, quality gates.

---

## 🤝 Contributing

PRs welcome. Two non-negotiables:
1. `cargo test` green
2. Templates carry the **four required placeholders** (`${positive_prompt}`, `${seed}`, `${width}`, `${height}`) — enforced at runtime

For themes / tags / workflow JSON, just edit the data files. No Rust changes needed.

---

## 📄 License

Dual-licensed under **MIT** or **Apache-2.0**, at your option.

---

## 🙏 Acknowledgements

- [Anima](https://civitai.com/models/2458426/anima) — CircleStone Labs + Comfy Org joint release, 2B text-to-image
- [ComfyUI](https://github.com/comfyanonymous/ComfyUI) — the workflow engine underneath
- [rig-core](https://github.com/0xPlaygrounds/rig) — clean LLM provider abstraction
- [cargo-zigbuild](https://github.com/rust-cross/cargo-zigbuild) — painless cross-compile from macOS to Linux

<div align="center">

<sub>Built with 🦀 in Rust. Talks to ComfyUI. Ships a 5.3 MB binary.</sub>

</div>