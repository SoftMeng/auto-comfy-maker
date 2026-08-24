# CLAUDE.md

This file provides guidance to Claude Code when working with code in this repository.

## 1. 工程铁律

1. **单一职责**：模块 / 函数 / 文档 / 命名 / 引用 / 边界 / 异常处理各自清晰；严禁越界、混合、混淆、命名混乱、引用混乱、归属不清、边界抽象、逻辑重复、异常静默处理。目标是正确性、可读性、可维护性、性能、安全。**不允许"打补丁 / 敷衍 / 捏造 / 臆测 / 留给下次"**。
2. **整洁精炼**：文档与代码篇幅**越精炼有效越好**，不敷衍也不过度设计。AI 倾向"加得越多错得越多"——能不写就不写，能合并就合并。

## 2. 文档编写约束

- **CLAUDE.md 自身**：保持 50-150 行，禁止成为"项目小抄"。详细工程参考下沉到 `docs/`。
- **设计文档**：单文件 ≤ 400 行；按职责拆分（架构 / CLI / 模板 / Prompt / 调度 / 配置）。
- **约束文档**：单篇 ≤ 200 行，规则必须是"可被工具或 PR 评审验证"的表述。
- **不在 CLAUDE.md 写**：阶段状态、ADR 简表、强制检查点（属于 Skill 协议）、未来规划。

## 3. 强制设计原则

1. **数据与逻辑分离**：ComfyUI workflow 是数据，不是代码；Rust 只负责"按节点 ID 替换参数"。
2. **两阶段 Prompt 解耦**：`combine()`（多维度 tag 拼接）与 `refine()`（LLM 优化）独立可调用，LLM 失败必须回退到拼接结果而非整体失败。
3. **配置分层**：`config/default.toml` 提交进仓库，`config/local.toml` 进 `.gitignore`，CLI 与环境变量覆盖最终值。
4. **错误传递用 `Result<T, E>`**：禁止 `unwrap()` 进入业务路径；库代码使用 `thiserror`，二进制入口使用 `anyhow`。
5. **CLI 命令三态明确**：`generate` 单次、`batch` 批量、`daemon` 定时，三者互不重叠，参数子集互不耦合。
6. **tags 文件即真相**：标签纯文本存储，每行一条；不引入数据库或索引文件，运行时按需加载到 `IndexSet<String>`。

## 4. 项目结构

```
auto-comfy-maker/
├── CLAUDE.md                  # 本文件
├── README.md                  # 读者入口
├── Cargo.toml                 # Rust 依赖与构建
├── config/                    # 配置（default.toml 提交；local.toml gitignore）
├── templates/                 # ComfyUI workflow JSON 模板
├── tags/                      # 多维度 prompt 标签（每文件一个维度）
├── src/                       # Rust 源码
│   ├── cli/                   # clap 命令定义
│   ├── config/                # 配置加载
│   ├── comfyui/               # ComfyUI HTTP 客户端
│   ├── prompt_engine/         # 标签组合 + LLM 优化
│   ├── scheduler/             # 定时任务与周期
│   └── main.rs                # 入口
├── docs/                      # 设计 + 约束文档
│   ├── README.md              # docs 导航
│   ├── architecture.md
│   ├── cli.md
│   ├── workflow-template.md
│   ├── prompt-engine.md
│   ├── scheduler.md
│   ├── config.md
│   └── constraint/            # 约束体系
├── output/                    # 生成的图片（gitignore）
└── logs/                      # 运行日志（gitignore）
```

## 5. 工程参考

| 维度 | 选择 | 说明 |
|------|------|------|
| 语言 | Rust 2021 edition | 内存安全、零成本异步 |
| 异步运行时 | tokio 1.x | ComfyUI HTTP 长轮询 |
| HTTP 客户端 | reqwest | 仅用于 ComfyUI REST / WebSocket |
| LLM 框架 | rig-core 0.40 | 统一 OpenAI / Anthropic provider；见 agent_cli |
| CLI | clap 4.x（derive 模式） | 子命令 + 自动 help |
| 配置 | serde + toml | 配置即代码、强类型 |
| 错误处理 | thiserror + anyhow | 库层 / 二进制层分离 |
| 时间处理 | chrono | cron 表达式友好 |
| 日志 | tracing + tracing-subscriber | 结构化日志 |

**架构图**：

```
┌─────────────────────────────────────────────────┐
│                  CLI (clap)                     │
│   generate │ batch │ daemon │ tags │ config    │
└────────────────────┬────────────────────────────┘
                     │
       ┌─────────────┼─────────────┐
       ▼             ▼             ▼
   scheduler     prompt_engine   comfyui client
   (tokio cron)  (combine→refine)(HTTP + WS)
       │             │             │
       └─────────────┼─────────────┘
                     ▼
              config loader
              (toml + env)
```

## 6. 文档索引

| 主题 | 路径 |
|------|------|
| 架构与数据流 | `docs/architecture.md` |
| CLI 参数表 | `docs/cli.md` |
| Workflow 模板替换机制 | `docs/workflow-template.md` |
| Prompt 两阶段生成 | `docs/prompt-engine.md` |
| 定时任务与周期 | `docs/scheduler.md` |
| 配置文件结构 | `docs/config.md` |
| 约束体系总览 | `docs/constraint/README.md` |
| 命名规范 | `docs/constraint/naming.md` |
| 目录结构约束 | `docs/constraint/structure.md` |
| 格式化规范 | `docs/constraint/format.md` |
| 提交规范 | `docs/constraint/commit.md` |
| 测试规范 | `docs/constraint/testing.md` |
| 质量门控 | `docs/constraint/quality.md` |

## 7. 快速命令

```bash
cargo build --release      # 构建发布版
cargo run -- generate      # 单次生成（参数见 docs/cli.md）
cargo run -- batch -n 10   # 一次性生成 10 张
cargo run -- daemon        # 启动定时任务
cargo test                 # 运行单元测试
cargo clippy -- -D warnings# Lint 检查（warning 视为错误）
cargo fmt --check          # 格式化检查
```

## 8. Skill 映射

| 任务类型 | 首选 Skill |
|----------|------------|
| 任务规划 | `/harness-plan` |
| Rust 开发 | `/harness-rust-development` |
| 文档结构 | `/harness-doc-design` |
| 代码审查 | `/harness-code-review` |
| 问题定位 | `/harness-debug` |
| 质量验证 | `/harness-quality-verification` |

## 9. 重要注意事项

- **ComfyUI URL**：从 `config/default.toml` 的 `[comfyui].url` 读取；支持环境变量 `COMFYUI_URL` 覆盖。
- **OpenAI 调用**：可选，关闭时直接使用方式 1 输出；失败时回退而非终止。
- **图片保存路径**：`output/{YYYY-MM-DD}/{timestamp}_{prompt-hash}.png`。
- **定时任务持久化**：计划列表存储于 `config/schedule.toml`，避免重复触发历史任务。
- **类似项目反模式（来自 glmclaw）**：CLAUDE.md 臃肿、文档职责不清、配置与代码耦合——本项目已规避。
- **参考实现（实现基础）**：`/Users/xiangyuanmeng/Documents/Qoder/agent_cli`（rig-core 0.40）。LLM provider 抽象直接采用其 `agent/builder.rs` 的 `AgentKind` / `ProviderKind` 模式。