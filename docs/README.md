# auto-comfy-maker · docs 导航

本目录是项目的设计文档与约束体系的**单一入口**。

## 这是什么

`auto-comfy-maker` 是一个基于 Rust 的**自动化图片生成工具**。它通过 ComfyUI（已部署）调用 workflow，并支持：

- **CLI 单次 / 批量生成**
- **定时任务模式**（interval / cron / 时刻列表）
- **多维度 tag 拼接**作为 prompt 基础
- **可选 OpenAI LLM 优化 prompt**

## 我应该先读什么

| 我想了解 | 先读 |
|---------|------|
| 这个项目**做什么 / 不做什么** | `../CLAUDE.md` |
| 整体架构与模块边界 | [`architecture.md`](./architecture.md) |
| CLI 命令与参数 | [`cli.md`](./cli.md) |
| ComfyUI 模板替换机制 | [`workflow-template.md`](./workflow-template.md) |
| prompt 怎么生成 | [`prompt-engine.md`](./prompt-engine.md) |
| 定时任务怎么跑 | [`scheduler.md`](./scheduler.md) |
| 配置怎么写 | [`config.md`](./config.md) |
| 命名 / 结构 / 格式 / 提交 / 测试 / 质量约束 | [`constraint/`](./constraint/) |

## 文档分类

### 设计文档（业务导向）

- `architecture.md`：模块依赖、数据流、关键决策。
- `cli.md`：命令、参数、互斥规则。
- `workflow-template.md`：模板替换算法与 MANIFEST 机制。
- `prompt-engine.md`：两阶段 prompt 生成。
- `scheduler.md`：三种调度模式与持久化。
- `config.md`：三层优先级与校验规则。

### 约束文档（执行导向）

- `constraint/README.md`：约束体系总览。
- `constraint/naming.md`：Rust 命名规范。
- `constraint/structure.md`：目录结构与模块边界。
- `constraint/format.md`：格式化与注释规范。
- `constraint/commit.md`：提交信息格式。
- `constraint/testing.md`：测试覆盖与组织。
- `constraint/quality.md`：CI 质量门控。

## 设计哲学

1. **数据与逻辑分离**：ComfyUI workflow 是数据，Rust 不解释业务语义。
2. **失败优雅降级**：LLM 失败 → 回退；单次任务失败 → 记录而非中断 daemon。
3. **配置即代码**：强类型 `AppConfig`，加载阶段校验完毕。
4. **约束可验证**：每条规则都有工具或评审可检查的执行方式。
5. **不重复造轮子**：tokio / clap / reqwest / tracing 优先于自实现。

## 与 glmclaw 的差异

glmclaw 是同类型但质量较差的 Node.js 项目。本项目规避其常见反模式：

- ✅ CLAUDE.md 控制在 130 行（glmlaw 数百行）。
- ✅ 模块边界清晰，配置与代码解耦。
- ✅ 文档按职责拆分，避免单文档臃肿。
- ✅ 错误处理有显式策略（Result + 回退）。