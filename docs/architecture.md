# 架构设计

## 结论

本项目是一个**单二进制 + tokio 运行时**的 Rust 应用。核心架构原则：**ComfyUI workflow 是数据，Rust 只做参数替换与轮询**；**两阶段 prompt 生成是组合 + 优化的流水线**；**调度器与 CLI 共用同一组核心 service**。

## 数据流（一张图）

```
用户命令
   │
   ▼
┌──────────┐    ┌──────────┐    ┌──────────────┐
│ CLI 层   │ ─▶ │ Config   │ ─▶ │ PromptEngine │
│ (clap)   │    │ (toml)   │    │ (combine→refine)│
└──────────┘    └──────────┘    └──────┬───────┘
                                        │  最终 prompt
                                        ▼
                              ┌──────────────────┐
                              │ ComfyUI Client   │
                              │ (reqwest)        │
                              └──────┬───────────┘
                                     │
                                     ▼
                              ┌──────────────────┐
                              │ workflow 模板替换 │ ◀── templates/
                              │ (serde_json)     │
                              └──────┬───────────┘
                                     │ 提交 prompt_id
                                     ▼
                              ┌──────────────────┐
                              │ 轮询生成结果     │
                              │ (WebSocket/HTTP) │
                              └──────┬───────────┘
                                     │  图片字节流
                                     ▼
                              ┌──────────────────┐
                              │ 落盘到 output/   │
                              └──────────────────┘
```

## 模块边界（强制）

| 模块 | 单一职责 | 入度 | 出度 |
|------|----------|------|------|
| `cli` | 解析参数、调度子命令 | 0 | config, scheduler, prompt_engine, comfyui |
| `config` | 加载 / 校验 toml | 0 | 0 |
| `comfyui` | HTTP 客户端（reqwest）+ workflow 模板替换 | 1 (config) | 0 |
| `prompt_engine` | tag 组合（combine）+ LLM 优化（refine via rig-core） | 1 (config) | 0 |
| `scheduler` | cron / interval / 持久化 | 1 (config) | 3 (prompt_engine, comfyui, scheduler-self) |
| `main.rs` | 组装依赖、初始化 tracing | all | 0 |

**规则**：
- `config` 不依赖任何模块（避免循环）。
- `comfyui` 不依赖 `prompt_engine`（HTTP 层无业务理解）。
- 跨模块调用必须经过 trait 抽象（便于 mock 测试）。

## 关键设计决策

### 决策 1：异步运行时统一为 tokio

**为什么**：ComfyUI 的图片生成需要长轮询 / WebSocket；同步阻塞会拖垮整个进程。tokio 是 Rust 异步的事实标准。

**代价**：业务逻辑必须显式 `async/.await`，错误传播链长一节。

### 决策 2：workflow 模板替换用 serde_json 而非手写字符串

**为什么**：ComfyUI workflow 是嵌套 JSON，手工拼接极易出错。serde_json 提供类型安全的 Value 操作（`pointer_mut`）。

**代价**：引入 `serde_json` 依赖；模板字段需在 `templates/MANIFEST.toml` 维护节点 ID 映射。

### 决策 3：两阶段 prompt 解耦为独立 trait

**为什么**：测试可以单独验证 `combine()` 的纯函数行为；`refine()` 失败时不污染上游结果。

**实现**：
```rust
pub trait PromptEngine: Send + Sync {
    fn combine(&self, ctx: &CombineContext) -> Result<String, PromptError>;
    async fn refine(&self, prompt: &str) -> Result<String, PromptError>;
}
```

### 决策 4：调度器使用 tokio::time + 自定义 cron 解析

**为什么**：`cron` 库依赖较重且不支持扩展语义；本项目只需要"分钟级"调度，自实现更易测试。

**代价**：不实现完整的 cron 表达式（不支持 `?`、`L`、`W`）。

### 决策 5：图片落盘按日期分目录

**为什么**：避免单目录文件过多导致文件系统瓶颈；便于按日期检索与归档。

### 决策 6：LLM client 用 rig-core，不自实现

**为什么**：rig-core 0.40 已统一 OpenAI / Anthropic provider 抽象，并提供 agent builder 模式（参考 `agent_cli/src/agent/builder.rs`）。自实现需要重复 provider 鉴权、流式响应、错误归一化等代码。

**边界**：rig 仅用于 LLM refine；ComfyUI REST / WebSocket 仍由 reqwest 处理（rig 不是通用 HTTP 客户端）。

## 反模式（来自 glmclaw 教训）

- ❌ 把 HTTP 客户端散布到 CLI 层（`main.rs` 直接 fetch）。
- ❌ 配置项散落在源代码与 toml 重复维护。
- ❌ 调度器与业务逻辑耦合在同一个 async 块里。
- ✅ 本项目强制模块边界，参见 `docs/constraint/structure.md`。