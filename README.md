<div align="center">

# 🎨 Auto Comfy Maker

**Rust 实现的自动化 ComfyUI 图片生成工具**

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust Version](https://img.shields.io/badge/rust-1.70+-orange.svg)](https://www.rust-lang.org/)
[![ComfyUI](https://img.shields.io/badge/ComfyUI-Compatible-blueviolet.svg)](https://github.com/comfyanonymous/ComfyUI)
[![Anima](https://img.shields.io/badge/Model-Anima_Ready-ff69b4.svg)]()
[![Build Status](https://img.shields.io/badge/build-passing-brightgreen.svg)]()
[![Tests](https://img.shields.io/badge/tests-24_passing-success.svg)]()

通过多维度标签组合 + 可选 LLM 优化 + Anima 模型深度适配，<br>
实现高效可控的 AI 图像生成。

[功能特性](#-功能特性) · [快速开始](#-快速开始) · [主题库](#-主题库) · [生成示例](#-生成示例) · [文档](#-文档)

</div>

---

## ✨ 项目亮点

<table>
<tr>
<td width="25%" align="center">

### 🎯 Anima 深度适配
完整支持 Danbooru 标签系统<br>
+ 艺术家（@）格式
</td>
<td width="25%" align="center">

### ⚡ CLI 三模式
单次（generate）<br>
批量（batch）/ 定时（daemon）
</td>
<td width="25%" align="center">

### 🎨 多主题系统
TOML 配置化主题<br>
+ 冲突规则保证一致性
</td>
<td width="25%" align="center">

### 🛡️ 工程级质量
配置分层 + 错误处理<br>
+ 24 个单元测试通过
</td>
</tr>
</table>

---

## 🚀 快速开始

### 📋 前置要求

| 依赖 | 版本 | 说明 |
|------|------|------|
| **Rust** | 1.70+ | 编译与运行 |
| **ComfyUI** | 最新 | 图像生成服务 |
| **Anima 模型** | v1.0+ | 推荐模型（可选） |

### ⚡ 三步上手

```bash
# 1️⃣ 克隆项目
git clone https://github.com/SoftMeng/auto-comfy-maker.git
cd auto-comfy-maker

# 2️⃣ 构建发布版本
cargo build --release

# 3️⃣ 生成第一张图片
cargo run -- generate --theme anima-drawing-v5 --lang en
```

> 💡 **提示**：首次运行前请检查 `config/default.toml` 中的 ComfyUI 地址。

---

## 📦 功能特性

### 🎮 CLI 三种模式

<table>
<tr>
<th width="33%">单次生成 generate</th>
<th width="33%">批量生成 batch</th>
<th width="33%">定时任务 daemon</th>
</tr>
<tr>
<td>

```bash
cargo run -- generate \
  --theme anima-drawing-v5 \
  --lang en
```

</td>
<td>

```bash
cargo run -- batch \
  -n 10 \
  --theme anima-drawing-v5
```

</td>
<td>

```bash
# 四种调度模式互斥使用
# 周期模式
cargo run -- daemon \
  --interval 6h --mode auto \
  --theme anima-drawing-v5

# 持续生成模式
cargo run -- daemon \
  --task-interval 5s --mode auto \
  --theme anima-simple
```

</td>
</tr>
</table>

### 🤖 Anima 模型深度适配

严格按照 [Anima 官方推荐](https://civitai.com/models/2458426/anima) 的 Danbooru 标签格式：

```text
[质量前缀] [1girl] [角色] [系列] [艺术家] [通用标签]

✅ masterpiece, best quality, score_7, safe, highres
✅ 1girl, solo, focus on single character
✅ @sakimichan, @wlop, @chinese artist
✅ children's book illustration, watercolor illustration
```

### ⏰ 调度器四种模式（互斥）

| 模式 | 参数 | 行为 | 适用场景 |
|------|------|------|----------|
| **周期模式** | `--interval 5m` | 每 5 分钟触发一次 tick | 定时任务 |
| **Cron 模式** | `--cron "0 */6 * * *"` | 定时调度 | 复杂时间规则 |
| **指定时刻** | `--at "2026-09-01 09:00:00"` | 一次性触发 | 未来执行 |
| **持续模式** | `--task-interval 5s` | 持续生成，任务后等 5s | 长期批量生成 |

### 📐 主题系统（Themes）

主题文件采用 TOML 格式，配置化定义 prompt 组合规则：

```toml
[order.fixed]
quality = { file = "tags/en/quality.txt", count = 1 }
character_count = { file = "tags/en/character_count.txt", count = 1 }
era = { file = "tags/en/era_only.txt", count = 1 }

[compatibility.conflicts]
era = [
  ["tang dynasty", "song dynasty", "ming dynasty"],
  ["heian period", "edo period", "meiji era"]
]
```

### 🏷️ 标签库（Tags）

按类别组织的标签文件，**支持中英文双语**：

```
tags/
├── zh/                      # 中文标签（18 个分类）
│   ├── 时代.txt            # 时代和地区
│   ├── 职业.txt            # 职业/阶层/角色
│   ├── 性格.txt            # 性格特征
│   ├── 世界观.txt          # 世界观设定
│   ├── 五官.txt            # 五官特征
│   ├── 发型.txt            # 发型
│   ├── 表情.txt            # 微表情/情绪
│   ├── 服装.txt            # 服装（按朝代分类）
│   ├── 配饰.txt            # 配饰
│   ├── 道具.txt            # 道具/武器
│   ├── 背景.txt            # 背景
│   ├── 姿态.txt            # 姿态
│   ├── 漫画类型.txt        # 漫画类型
│   ├── 画风.txt            # 画风
│   ├── 光线.txt            # 光线
│   ├── 色调.txt            # 色调
│   ├── 构图.txt            # 构图
│   ├── 氛围.txt            # 氛围
│   ├── 身体.txt            # 身体特征
│   └── 妆容.txt            # 妆容
└── en/                      # English tags (Danbooru style)
    ├── quality.txt          # masterpiece, best quality, score_7
    ├── character_count.txt  # 1girl, solo
    ├── era_only.txt         # tang dynasty, heian period
    ├── region.txt           # jiangnan water town china
    ├── archetype.txt        # scholar daughter, shrine maiden
    ├── clothing.txt         # ruqun, kimono, hanbok
    ├── hairstyle.txt        # long hair, twin tails
    ├── features.txt         # eyes, expressions, body
    ├── pose.txt             # standing, sitting
    ├── background.txt       # tang dynasty changan
    ├── art_style.txt        # children's book illustration
    ├── artists.txt          # @sakimichan, @wlop
    ├── profession.txt       # 职业
    ├── worldview.txt        # 世界观
    ├── personality.txt      # 性格
    ├── accessories.txt      # 配饰
    ├── props.txt            # 武器/道具
    └── _simple: anima-simple 主题专用
        ├── quality_simple.txt
        ├── character_count_simple.txt
        ├── character_simple.txt
        ├── artist_fixed.txt        # @Jang Chan 固定
        ├── required_simple.txt     # 强制高跟鞋
        ├── body_simple.txt
        ├── pose_simple.txt
        ├── view_simple.txt
        ├── background_simple.txt
        └── style_simple.txt
```

---

## 🛠️ 技术栈

<div align="center">

| ![Rust](https://img.shields.io/badge/Rust-2021%20Edition-orange.svg) | ![Tokio](https://img.shields.io/badge/Tokio-1.x-blue.svg) | ![rig-core](https://img.shields.io/badge/rig--core-0.40-purple.svg) |
|:---:|:---:|:---:|
| ![reqwest](https://img.shields.io/badge/reqwest-HTTP-green.svg) | ![clap](https://img.shields.io/badge/clap-4.x-red.svg) | ![serde](https://img.shields.io/badge/serde-toml-yellow.svg) |

</div>

### 📊 核心依赖

| 依赖 | 用途 | 选型理由 |
|------|------|---------|
| **Rust 2021** | 编程语言 | 内存安全、零成本异步 |
| **Tokio 1.x** | 异步运行时 | ComfyUI HTTP 长轮询 |
| **rig-core 0.40** | LLM 框架 | 统一 OpenAI / Anthropic |
| **reqwest** | HTTP 客户端 | ComfyUI REST / WebSocket |
| **clap 4.x** | CLI 解析 | 子命令 + 自动 help |
| **serde + toml** | 配置序列化 | 强类型配置 |
| **thiserror + anyhow** | 错误处理 | 库 / 二进制分离 |
| **tracing** | 日志 | 结构化日志 |

---

## 🎨 主题库

### 📌 内置主题

| 主题 ID | 名称 | 说明 |
|---------|------|------|
| `anima-drawing-v5` | **Anima Drawing v5** | 原创亚洲女性角色（Anima 深度适配） |
| `anima-simple` | **Anima Simple** | 极简标签风格，强制 1girl + @Jang Chan + 极高跟 |
| `portrait` | 人像主题 | 单人物像（中文） |
| `portrait-en` | Portrait (English) | 英文变体 |

### 🌟 Anima Drawing v5 特色

- ✅ **完全 Anima 官方格式**：[quality] [1girl] [character] [series] [artist] [general]
- ✅ **时代一致性**：避免跨时代服装混搭（inter 互斥规则）
- ✅ **丰富的 tags 库**：18 个中文 + 17 个英文分类文件
- ✅ **BTreeMap 字段排序**：保证 prompt 元素顺序稳定

### 🌟 Anima Simple 特色

- ✅ **极简风格**：每个 prompt 必含 1girl + @Jang Chan + 极高跟
- ✅ **持续生成友好**：搭配 `--task-interval` 适合长期批量生成
- ✅ **明确语义**：用户知道每次会生成什么

---

## 📸 生成示例

> 以下示例均由 `anima-drawing-v5` 主题自动生成，使用相同的 Anima 模型与冲突规则。读者可复制下方 prompt 自行复现。

### 🌸 民国女学生 · Scholar Daughter

![民国女学生](docs/assets/examples/republic-era-student.jpg)

```text
masterpiece, best quality, score_7, safe, highres, absurdres,
1girl, solo, focus on single character, dynasty era,
mandarin square, song dynasty hangzhou west lake,
scholar daughter, @ciloranko, medium hair, arms behind head
```

**主题**：`anima-drawing-v5`
**关键标签**：`mandarin square` · `scholar daughter` · `@ciloranko`

---

### 🌺 韩服角色 · Apprentice Geisha

![韩服角色](docs/assets/examples/hanbok-character.jpg)

```text
masterpiece, best quality, score_7, safe, highres, absurdres,
1girl, solo, focus on single character, goryeo dynasty,
wide obi, edo yoshiwara pleasure quarter, apprentice geisha,
@vagabond, hair down, seiza
```

**主题**：`anima-drawing-v5`
**关键标签**：`wide obi` · `apprentice geisha` · `@vagabond`

---

### 👠 高跟鞋角色 · Office Lady

![高跟鞋](docs/assets/examples/high-heels-character.jpg)

```text
masterpiece, best quality, score_7, safe, highres, absurdres,
1girl, solo, focus on single character, meiji era,
office lady suit, fog, magical girl,
@chinese artist, braided bun, standing
```

**主题**：`anima-drawing-v5`
**关键标签**：`office lady suit` · `braided bun` · `@chinese artist`

---

### 🏰 现代场景 · Cafe Owner

![现代场景](docs/assets/examples/modern-scene.jpg)

```text
masterpiece, best quality, score_7, safe, highres, absurdres,
1girl, solo, focus on single character, steampunk world,
twelve-layer robe, himalayan foothill monastery, cafe owner,
@ice, ponytail, cross-legged
```

**主题**：`anima-drawing-v5`
**关键标签**：`twelve-layer robe` · `cafe owner` · `@ice`

---

## 📚 文档

| 主题 | 路径 | 说明 |
|------|------|------|
| **Claude Code 协作指南** | [CLAUDE.md](./CLAUDE.md) | AI 辅助开发规范与工程铁律 |
| **架构与数据流** | [docs/architecture.md](./docs/architecture.md) | 系统架构、模块划分、数据流转 |
| **CLI 参数表** | [docs/cli.md](./docs/cli.md) | 完整命令参数说明 |
| **Prompt 生成引擎** | [docs/prompt-engine.md](./docs/prompt-engine.md) | 两阶段 Prompt 生成机制 |
| **Workflow 模板** | [docs/workflow-template.md](./docs/workflow-template.md) | ComfyUI JSON 模板替换规则 |
| **定时任务与周期** | [docs/scheduler.md](./docs/scheduler.md) | 调度模式与持久化 |
| **配置文件结构** | [docs/config.md](./docs/config.md) | TOML 配置分层机制 |
| **约束体系总览** | [docs/constraint/README.md](./docs/constraint/README.md) | 命名 / 结构 / 格式规范 |

---

## ⏰ 调度器示例

### 周期模式

```bash
# 每 6 小时生成一张
cargo run -- daemon --interval 6h --mode auto --theme anima-drawing-v5
```

### Cron 模式

```bash
# 每天 9:00 执行
cargo run -- daemon --cron "0 9 * * *" --mode fixed --prompt "1girl, masterpiece"
```

### 持续生成模式

```bash
# 每张图后等 5s，持续生成
cargo run -- daemon --task-interval 5s --mode auto --theme anima-simple
```

### 指定时刻

```bash
# 9 月 1 日 9 点执行
cargo run -- daemon --at "2026-09-01 09:00:00" --mode fixed --prompt "..."
```

---

## 📊 项目状态

### 🚧 当前阶段

**核心功能已就绪**，可投入实际使用

| 模块 | 状态 | 说明 |
|------|------|------|
| **核心框架** | ✅ | Rust + Tokio + ComfyUI 集成 |
| **Anima Drawing v5** | ✅ | Danbooru 标签 + 冲突规则 |
| **Anima Simple** | ✅ | 极简风格主题 |
| **中文 tags** | ✅ | 18 个分类文件 |
| **英文 tags** | ✅ | 17 个 Danbooru 风格文件 |
| **ComfyUI 集成** | ✅ | HTTP 提交 + 轮询 + 下载 |
| **LLM 优化** | ✅ | OpenAI / Anthropic 适配 |
| **调度器** | ✅ | 4 种互斥模式（interval/cron/at/task-interval） |
| **单元测试** | ✅ | 24 个全部通过 |

### 📈 路线图

- [ ] 增加更多 Anima 主题（男性角色、风景、概念艺术等）
- [ ] 支持 LLM 实时优化 prompt
- [ ] Web UI 仪表板
- [ ] 主题市场（社区共享）

---

## 🤝 贡献

欢迎提交 Issue 和 Pull Request！

```bash
# Fork 项目
# 创建特性分支
git checkout -b feature/amazing-feature

# 提交变更
git commit -m "feat: add amazing feature"

# 推送到分支
git push origin feature/amazing-feature

# 创建 Pull Request
```

### 📋 贡献指南

1. 遵循 `docs/constraint/` 中的工程规范
2. 主题文件必须经过实际生成验证
3. 新增 tags 必须中英文双语同步
4. 提交前运行 `cargo test` 确保测试通过

---

## 📄 License

MIT License - 详见 [LICENSE](LICENSE) 文件

---

## 🙏 致谢

- [Anima](https://civitai.com/models/2458426/anima) - CircleStone Labs + Comfy Org 联合发布的 2B 文本到图像模型
- [ComfyUI](https://github.com/comfyanonymous/ComfyUI) - 强大的 Stable Diffusion GUI
- [rig-core](https://github.com/0xPlaygrounds/rig) - 统一 LLM Provider 抽象
- 所有为本项目贡献代码和主题的开发者

---

<div align="center">

**Built with ❤️ using Rust and ComfyUI**

[⬆ 回到顶部](#-auto-comfy-maker)

</div>