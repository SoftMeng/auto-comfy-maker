# auto-comfy-maker

<div align="center">

**Rust 实现的自动化 ComfyUI 图片生成工具**

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust Version](https://img.shields.io/badge/rust-1.70+-orange.svg)](https://www.rust-lang.org/)
[![Build Status](https://img.shields.io/badge/build-passing-brightgreen.svg)]()

通过多维度标签组合 prompt 与可选 LLM 优化，实现高效可控的 AI 图像生成。

[功能特性](#-功能特性) · [快速开始](#-快速开始) · [文档](#-文档) · [项目状态](#-项目状态)

</div>

---

## 🚀 快速开始

### 前置要求

- Rust 1.70+
- ComfyUI 服务运行中

### 三步上手

```bash
# 1. 克隆项目
git clone https://github.com/SoftMeng/auto-comfy-maker.git
cd auto-comfy-maker

# 2. 构建发布版本
cargo build --release

# 3. 生成第一张图片
cargo run -- generate --lang zh
```

**💡 提示**：首次运行前请配置 `config/default.toml` 中的 ComfyUI 地址。

---

## ✨ 项目亮点

<div align="center">

| 🎯 **精准控制** | ⚡ **灵活高效** | 🔧 **可扩展** | 🛡️ **工程级质量** |
|---------------|---------------|-------------|----------------|
| 多维度标签系统，精确描述图像生成需求 | CLI 三模式，支持单次/批量/定时任务 | ComfyUI JSON 模板化，易于自定义工作流 | 配置分层、错误处理、结构化日志 |

</div>

---

## 📦 功能特性

### CLI 三模式

```bash
# 单次生成
cargo run -- generate --lang zh --theme anime

# 批量生成
cargo run -- batch -n 10 --lang en

# 定时任务
cargo run -- daemon --schedule "0 */6 * * *"
```

### 多维度 Prompt 引擎

```toml
# 支持中英文标签体系
tags/
├── zh/          # 中文标签（发型、场景、风格...）
└── en/          # 英文标签

themes/
└── anime.toml   # 主题配方（如何组合标签）
```

### ComfyUI 模板集成

```json
{
  "3": {
    "inputs": {
      "text": "${prompt}"
    }
  }
}
```

按节点 ID 替换参数，支持任意 ComfyUI 工作流。

### 可选 LLM 优化

```toml
[llm]
provider = "openai"  # 或 "anthropic"
model = "gpt-4"
enabled = true
```

失败时自动回退到拼接结果，确保可用性。

---

## 🛠️ 技术栈

<div align="center">

[![Rust](https://img.shields.io/badge/Rust-2021%20Edition-orange.svg)](https://www.rust-lang.org/)
[![Tokio](https://img.shields.io/badge/Tokio-1.x-blue.svg)](https://tokio.rs/)
[![rig-core](https://img.shields.io/badge/rig--core-0.40-purple.svg)](https://github.com/0xPlaygrounds/rig)
[![reqwest](https://img.shields.io/badge/reqwest-HTTP-green.svg)](https://github.com/seanmonstar/reqwest)

</div>

<div align="center">

**核心依赖** • 内存安全 • 异步 HTTP • LLM 抽象

</div>

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
| **约束体系总览** | [docs/constraint/README.md](./docs/constraint/README.md) | 命名/结构/格式规范 |

---

## 📊 项目状态

### 当前阶段

🚧 **项目初始化阶段**

当前提交包含：
- ✅ 完整文档体系与目录骨架
- ✅ 配置文件模板
- ✅ CLI 参数定义
- ⏳ 源码实现（`src/`、`Cargo.toml`）待完成

### 参考实现

- [agent_cli](https://github.com/...) - rig-core 0.40 LLM 集成样例

### License

MIT License - 详见 [LICENSE](LICENSE) 文件

---

## 🤝 贡献

欢迎提交 Issue 和 Pull Request！

---

<div align="center">

**Built with ❤️ using Rust and ComfyUI**

[⬆ 回到顶部](#auto-comfy-maker)

</div>