<div align="center">

# 🎨 Auto Comfy Maker

**Rust 实现的自动化 ComfyUI 图片生成工具**

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust Version](https://img.shields.io/badge/rust-1.70+-orange.svg)](https://www.rust-lang.org/)
[![ComfyUI](https://img.shields.io/badge/ComfyUI-Compatible-blueviolet.svg)](https://github.com/comfyanonymous/ComfyUI)
[![Anima](https://img.shields.io/badge/Model-Anima_Ready-ff69b4.svg)]()
[![Build Status](https://img.shields.io/badge/build-passing-brightgreen.svg)]()

通过多维度标签组合 + 可选 LLM 优化 + Anima 模型深度适配，<br>
实现高效可控的 AI 图像生成。

[功能特性](#-功能特性) · [快速开始](#-快速开始) · [主题库](#-主题库) · [文档](#-文档) · [生成示例](#-生成示例)

</div>

---

## ✨ 项目亮点

<table>
<tr>
<td width="25%" align="center">

### 🎯 精准控制
多维度标签系统<br>
精确描述图像细节
</td>
<td width="25%" align="center">

### ⚡ 灵活高效
CLI 三模式<br>
单次 / 批量 / 定时
</td>
<td width="25%" align="center">

### 🤖 Anima 适配
原生支持 Danbooru 标签<br>
+ 艺术家（@）格式
</td>
<td width="25%" align="center">

### 🛡️ 工程级质量
配置分层 + 错误处理<br>
+ 7 个单元测试通过
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

### 🎮 CLI 三模式

<table>
<tr>
<th width="33%">单次生成</th>
<th width="33%">批量生成</th>
<th width="33%">定时任务</th>
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
cargo run -- daemon \
  --schedule "0 */6 * * *"
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

按类别组织的标签文件，最小词元，**支持中英文双语**：

```
tags/
├── zh/                      # 中文标签
│   ├── 时代.txt            # Tang Dynasty / Heian period
│   ├── 职业.txt            # Profession / Social class
│   ├── 性格.txt            # Personality
│   ├── 世界观.txt          # World setting
│   ├── 五官.txt            # Facial features
│   ├── 发型.txt            # Hairstyle
│   ├── 服装.txt            # Clothing by dynasty
│   ├── 配饰.txt            # Accessories
│   ├── 道具.txt            # Props & weapons
│   ├── 背景.txt            # Background
│   ├── 姿态.txt            # Pose
│   ├── 漫画类型.txt        # Manga genre
│   ├── 画风.txt            # Art style
│   ├── 光线.txt            # Lighting
│   ├── 色调.txt            # Color tone
│   ├── 构图.txt            # Composition
│   ├── 氛围.txt            # Vibe
│   ├── 身体.txt            # Body features
│   └── 妆容.txt            # Makeup
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
    └── artists.txt          # @sakimichan, @wlop
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
| `anima-drawing-v5` | **Anima Drawing v5** | 原创亚洲女性角色（Anima 优化） |
| `portrait` | 人像主题 | 单人物像（中文） |
| `portrait-en` | Portrait (English) | 英文变体 |

### 🌟 Anima Drawing v5 特色

- ✅ **23 个角色维度**：时代、地域、性格、世界观、五官、发型、服装、配饰、道具、背景、姿态、艺术风格、艺术家等
- ✅ **时代一致性规则**：避免跨时代服装混搭
- ✅ **Danbooru 标签格式**：完全符合 Anima 官方要求
- ✅ **冲突互斥组**：同组元素不会同时出现

---

## 📸 生成示例

### 单角色作品

<details>
<summary><b>点击展开：示例 1 - 民国风女学生</b></summary>

**Prompt**：
```
1girl, solo, republic of china, k-pop stage outfit, masterpiece, best quality, score_7, safe, highres, absurdres, jeonju hanok village, geisha, @chinese artist, hair down, wide shot, grey hair
```

![示例1](output/2026-08-24/20260824-154737_4e6a05e2.jpg)

</details>

<details>
<summary><b>点击展开：示例 2 - 宋代文艺女</b></summary>

**Prompt**：
```
Cambodian Angkor Wat, moat reflection at dawn, Song dynasty beizi (褙子) long vest over inner garment, Blue Archive character art, cute and colorful, Qing Dynasty early(1644-1796), Incheon Chinatown, Korea, last princess of a conquered kingdom, ...
```

![示例2](output/2026-08-24/20260824-151956_9e921ed5.jpg)

</details>

---

## 📚 文档

| 主题 | 路径 | 说明 |
|------|------|------|
| **Claude Code 协作指南** | [CLAUDE.md](./CLAUDE.md) | AI 辅助开发规范与工程铁律 |
| **架构与数据流** | [docs/architecture.md](./docs/architecture.md) | 系统架构、模块划分、数据流转 |
| **CLI 参数表** | [docs/cli.md](./docs/cli.md) | 完整命令参数说明 |
| **Prompt 生成引擎** | [docs/prompt-engine.md](./docs/prompt-engine.md) | 两阶段 Prompt 生成机制 |
| **Workflow 模板** | [docs/workflow-template.md](./docs/workflow-template.md) | ComfyUI JSON 模板替换规则 |
| **定时任务与周期** | [docs/scheduler.md](./docs/scheduler.md) | Cron 表达式与任务调度 |
| **配置文件结构** | [docs/config.md](./docs/config.md) | TOML 配置分层机制 |
| **约束体系总览** | [docs/constraint/README.md](./docs/constraint/README.md) | 命名 / 结构 / 格式规范 |

---

## 📊 项目状态

### 🚧 当前阶段

**项目初始化阶段** —— 核心架构已就绪，可扩展更多主题

| 模块 | 状态 | 说明 |
|------|------|------|
| **核心框架** | ✅ | Rust + Tokio + ComfyUI 集成 |
| **Anima 主题** | ✅ | Danbooru 标签 + 冲突规则 |
| **中文 tags** | ✅ | 18 个分类文件 |
| **英文 tags** | ✅ | 11 个 Danbooru 风格文件 |
| **ComfyUI 集成** | ✅ | HTTP 提交 + 轮询 + 下载 |
| **LLM 优化** | ✅ | OpenAI / Anthropic 适配 |
| **单元测试** | ✅ | 7 个 workflow 测试通过 |
| **批量 / 定时** | 🚧 | 命令已就绪，待增强 |

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