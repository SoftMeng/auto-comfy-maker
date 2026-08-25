<div align="center">

# 🎨 Auto Comfy Maker

**把标签库 + 主题变成 ComfyUI 可直接用的 prompt — 自动完成。**

[English](./README.md) · **简体中文**

</div>

<div align="center">

[![License: MIT OR Apache-2.0](https://img.shields.io/badge/License-MIT%20OR%20Apache--2.0-blue.svg)](./LICENSE)
[![Rust 2021](https://img.shields.io/badge/Rust-2021%20Edition-orange.svg)](https://www.rust-lang.org/)
[![ComfyUI](https://img.shields.io/badge/ComfyUI-Compatible-blueviolet.svg)](https://github.com/comfyanonymous/ComfyUI)
[![Tests](https://img.shields.io/badge/tests-70_passing-success.svg)](#-开发)
[![Cross compile](https://img.shields.io/badge/Linux_x86__64-static-lightgrey.svg)](#-部署)

</div>

---

## 为什么？

你有一个 **ComfyUI 服务**和一堆**标签**，想把它们拼成 prompt。

手动写 prompt 写一次挺有意思——写一百次就不是了。Auto Comfy Maker 帮你

- 从多个维度池里抽标签，
- 组合成稳定的 prompt（带冲突、权重、可选 LLM 润色），
- 注入到 ComfyUI 的 workflow JSON 里，
- 提交、轮询、下载——你不用写一行胶水代码。

5 分钟入门：写一个主题 → 运行 `generate` → 拿到图片。

---

## ✨ 特性

| | |
|---|---|
| 🏷️ **标签即数据** | 标签就是纯文本文件，按维度分组。随便用编辑器改。 |
| 🎨 **主题系统** | 一份 TOML = 一种风格。声明 `fixed / random / optional` 槽位 + 冲突规则。 |
| 🔗 **可组合** | 多个主题混用、批量中途换 prompt，无需改代码。 |
| 🤖 **LLM 可选** | 用 OpenAI / Anthropic 润色（基于 [rig-core](https://github.com/0xPlaygrounds/rig)）。失败自动回退。 |
| 📡 **ComfyUI 原生** | 提交 workflow JSON、轮询、下载 PNG——和 ComfyUI 的"Queue Prompt"一模一样。 |
| ⏰ **四种调度模式** | `interval` · `cron` · `at` · `task-interval`——前三种互斥；`task-interval` 单用合法 |
| 📦 **单一静态二进制** | `cargo-zigbuild` 交叉编译出 5.3 MB 的全静态 Linux 二进制。 |
| 🛡️ **配置分层** | `default.toml`（进仓库） + `local.toml`（gitignore） + 环境变量。 |

---

## 🚀 快速开始

> [!NOTE]
> 假设你的 ComfyUI 服务跑在 `http://127.0.0.1:8188`（默认地址）。通过 `config/local.toml` 或环境变量 `COMFYUI_URL` 修改。

```bash
# 1. 构建（release 版）
cargo build --release

# 2. 用内置 Anima 主题生成一张图
./target/release/auto-comfy-maker generate \
  --theme anima-simple \
  --lang en

# 3. 或者批量随机组合 10 张
./target/release/auto-comfy-maker batch -n 10 --theme anima-drawing-v5

# 4. 或者每小时自动生成的守护进程
./target/release/auto-comfy-maker daemon --interval 1h --mode auto --theme anima-simple
```

生成的图片通常落在 `output/<YYYY-MM-DD>/<timestamp>_<hash>.png`。

---

## 🎨 主题一览

主题是**声明式配方**——告诉引擎从哪些标签维度抽样、每个抽多少、避免哪些冲突。

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

内置主题：

| ID | 风格 | 语言 |
|---|---|---|
| `anima-simple` | 极简，1girl + 固定艺术家 | `en` |
| `anima-drawing-v5` | 完整 Danbooru，含朝代冲突规则 | `en` |
| `portrait` | 中文古风人物 | `zh` |
| `portrait-en` | 英文人像变体 | `en` |

> [!TIP]
> 想加新风格？往 `themes/` 丢一个 `<你的风格>.toml` 就行。不用动代码。字段说明见 `themes/README.md`。

---

## 🔌 Workflow 模板

ComfyUI workflow 是 **JSON 数据**，不是代码。从 ComfyUI 导出 workflow，扔进 `templates/`，把注入点写成 `${positive_prompt}` / `${seed}` / `${width}` / `${height}`（或中文别名 `${提示词}` 等），就完事了。

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
> 每个模板加载时都会校验 —— 四个占位符缺任意一个，立刻报错并列出缺失字段。绝不会有"字面量偷偷漏到 ComfyUI"这种事。

内置模板：`anima`、`anima-aesthetic`、`anima-lora`、`zimage`。

---

## 📦 CLI 速查

```text
auto-comfy-maker generate   # 单张图，单主题，手动控制
auto-comfy-maker batch      # N 张图，随机组合
auto-comfy-maker daemon     # interval / cron / at / task-interval 四选一
auto-comfy-maker tags       # list / show / add / remove 标签
auto-comfy-maker config     # 展示合并后的配置 / 校验
```

<details>
<summary><b>generate</b> — 单张图，精确控制</summary>

```bash
auto-comfy-maker generate \
  --theme anima-simple \
  --template anima-lora \
  --lang en \
  --seed 42 \
  --width 768 --height 1536 \
  --no-send   # 只打印 prompt，不提交给 ComfyUI
```

</details>

<details>
<summary><b>batch</b> — N 张随机组合</summary>

```bash
auto-comfy-maker batch -n 20 --theme anima-simple --lang en
```

</details>

<details>
<summary><b>daemon</b> — 四种互斥模式</summary>

| 模式 | 参数 | 示例 |
|---|---|---|
| 周期 | `--interval 30m` | 每 30 分钟 |
| Cron | `--cron "0 9 * * *"` | 每天 09:00 |
| 指定时刻 | `--at 2026-09-01T09:00:00+08:00` | 一次性定时 |
| 持续任务间隔 | `--task-interval 5s` | 持续生成，每张间隔 5s |

守护进程持久化到 `config/schedule.toml`（gitignore）——重启不丢任务、不重复触发。

</details>

完整参数表：[`docs/cli.md`](./docs/cli.md)。

---

## 🧱 技术栈

| 层 | 选型 | 理由 |
|---|---|---|
| 语言 | Rust 2021 | 内存安全、零成本异步 |
| 运行时 | Tokio 1.x | ComfyUI HTTP 长轮询 |
| LLM | rig-core 0.40 | 统一 OpenAI / Anthropic 抽象 |
| HTTP | reqwest | 仅 ComfyUI REST |
| CLI | clap 4 (derive) | 子命令 + 自动生成 `--help` |
| 配置 | serde + toml | 强类型配置 |
| 错误 | thiserror（库） + anyhow（bin） | 按层清晰分离 |
| 日志 | tracing + tracing-subscriber | 结构化、按级别过滤 |

> [!NOTE]
> 没有数据库。没有消息队列。没有 Web 框架。就是一个二进制，跟 ComfyUI 对话，写 PNG。

---

## 📦 部署

### macOS → Linux（静态二进制）

```bash
# 一次性安装交叉编译工具链
brew install zig
cargo install cargo-zigbuild
export PATH="$HOME/.cargo/bin:$PATH"

# 编译（如果在国内网络，加代理）
export https_proxy=http://127.0.0.1:7890 \
       http_proxy=http://127.0.0.1:7890 \
       all_proxy=socks5://127.0.0.1:7890

cargo zigbuild --release --target x86_64-unknown-linux-musl
# → target/x86_64-unknown-linux-musl/release/auto-comfy-maker (5.3 MB, 全静态链接)
```

### 用 systemd 跑守护进程

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

## 📚 文档

| | |
|---|---|
| 🏛 [架构与数据流](./docs/architecture.md) | 模块如何组合 |
| 🎮 [CLI 参数表](./docs/cli.md) | 每个 flag、每个默认值 |
| 🧬 [Prompt 引擎](./docs/prompt-engine.md) | combine() → refine() 两阶段 |
| ⏰ [调度器](./docs/scheduler.md) | 四种 daemon 模式详解 |
| 🧩 [Workflow 模板](./docs/workflow-template.md) | 占位符契约 |
| ⚙️ [配置分层](./docs/config.md) | default / local / env 优先级 |
| 🤖 [AI 协作指南（CLAUDE.md）](./CLAUDE.md) | AI 辅助开发的约定 |
| 🛣 [路线图](./docs/ROADMAP.md) | 未来方向 |

---

## 🛠 开发

```bash
cargo build            # 调试版
cargo test             # 70 个单元测试，全绿
cargo clippy -- -D warnings
cargo fmt --check
```

工程规范见 [`docs/constraint/`](./docs/constraint/README.md)：命名、结构、格式、提交、测试、质量门控。

---

## 🤝 贡献

欢迎 PR。两条硬性要求：

1. `cargo test` 全绿
2. 模板必须含**四个必需占位符**（`${positive_prompt}`、`${seed}`、`${width}`、`${height}`）——运行时强制校验

主题 / 标签 / workflow JSON 只改数据文件即可，**无需 Rust 代码改动**。

---

## 📄 许可证

双协议：**MIT** 或 **Apache-2.0**，任你选。

---

## 🙏 致谢

- [Anima](https://civitai.com/models/2458426/anima) — CircleStone Labs + Comfy Org 联合发布的 2B 文生图模型
- [ComfyUI](https://github.com/comfyanonymous/ComfyUI) — 底层的 workflow 引擎
- [rig-core](https://github.com/0xPlaygrounds/rig) — 干净的 LLM provider 抽象
- [cargo-zigbuild](https://github.com/rust-cross/cargo-zigbuild) — macOS → Linux 无痛交叉编译

<div align="center">

<sub>用 🦀 Rust 编写，跟 ComfyUI 对话，吐出 5.3 MB 的二进制。</sub>

</div>