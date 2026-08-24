# auto-comfy-maker

Rust 实现的自动化图片生成工具。通过 ComfyUI 生成图片，支持多维度标签组合 prompt 与可选 LLM 优化。

## 功能

- **CLI**：单次（`generate`）、批量（`batch`）、定时任务（`daemon`）三种模式。
- **多维度 prompt**：中文 / 英文两套标签，按维度（发型、首饰、场景…）拆分。
- **ComfyUI 集成**：JSON 模板按节点 ID 替换参数。
- **LLM 优化**（可选）：通过 [rig-core](https://github.com/0xPlaygrounds/rig) 接入 OpenAI / Anthropic，失败时回退到拼接结果。
- **配置分层**：default.toml + local.toml + 环境变量。

## 快速开始

```bash
git clone https://github.com/SoftMeng/auto-comfy-maker.git
cd auto-comfy-maker
cargo build --release
cargo run -- generate --lang zh
```

## 文档

| 主题 | 路径 |
|------|------|
| Claude Code 协作指南 | [CLAUDE.md](./CLAUDE.md) |
| 架构与数据流 | [docs/architecture.md](./docs/architecture.md) |
| CLI 参数 | [docs/cli.md](./docs/cli.md) |
| Prompt 生成 | [docs/prompt-engine.md](./docs/prompt-engine.md) |
| 定时任务 | [docs/scheduler.md](./docs/scheduler.md) |
| 配置文件 | [docs/config.md](./docs/config.md) |
| 约束体系 | [docs/constraint/](./docs/constraint/) |

## 状态

🚧 **项目初始化阶段**。当前提交仅含文档与目录骨架，源码（`src/`、`Cargo.toml`）待实现。

## 参考实现

- [agent_cli](https://github.com/...)：rig-core 0.40 使用样例。

## License

待定。