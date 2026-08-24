# Roadmap

## v0.1.0（已发布）

P0–P5 全部完成：

- 文档：CLAUDE.md + 7 篇设计 + 7 篇约束
- 工程：Rust 2021 + tokio + reqwest + rig-core 0.40 + serde + tracing
- 模块：`cli / config / theme / tags / prompt_engine / comfyui / scheduler`
- 子命令：`generate / batch / daemon / tags / config`
- 测试：59 个单元测试全绿；`cargo clippy -- -D warnings` 零警告
- CI：fmt / clippy / test / build / audit 五 job
- 安全：`config/local.toml` 在 `.gitignore`，无需 `config.yaml.example`（项目仅用 toml）

## v0.2.0（候选）

短期可独立完成的改进，按 ROI 排序：

### 1. 健康检查接入 CLI

**背景**：v0.1.0 的 `ComfyuiClient::system_stats` 在死代码清理中被删除（v0.1 阶段未使用）。
**方案**：新增 `auto-comfy-maker doctor` 子命令，调用 `GET /system_stats` 检查 ComfyUI 连通性 + LLM API key 可用性。
**ROI**：中（用户首次使用前手动检查，节省排查时间）。

### 2. WebSocket 替换轮询

**背景**：当前 `poll_until_ready` 每 2s 轮询 `GET /history/{id}`。
**方案**：用 `WS /ws` 接收 ComfyUI 推送的 `executing` / `execution_success` 事件，省去轮询延迟与 HTTP 开销。
**ROI**：中（生产场景下，单图生成延迟从 ~3s 降到 ~50ms）。
**参考**：ComfyUI 官方 WS API `queue_remaining` / `execution_start` / `execution_success`。

### 3. tags 文件热加载

**背景**：当前 daemon 启动时加载一次 tags，运行期修改需重启。
**方案**：监听 tags 目录 `notify` crate，文件变化时重新加载对应 `TagStore`。
**ROI**：低（用户场景多为静态 tags）。

### 4. 流式 LLM 输出

**背景**：当前 `refine()` 用 `agent.prompt(text).await` 等完整结果。
**方案**：用 `agent.stream_prompt(...).await` 流式消费，首 token 到达即转发到 stdout，提升用户感知响应。
**ROI**：中（交互式 `generate` 体验提升；daemon 场景无感）。

### 5. theme 嵌套子分类

**背景**：当前 theme 仅支持 flat category（`order.fixed/random/optional`）。
**方案**：参考 glmclaw 的 `NestedCategory`，支持 `clothing.top/bottom/shoes` 子分类；`combine` 内部按子分类独立采样。
**ROI**：高（更精细的人物/服装控制）。
**风险**：复杂度上升，建议仅在有真实主题需求时引入。

### 6. 覆盖率强制

**背景**：当前 `docs/constraint/testing.md` 设定 70% 核心模块目标，但无 CI 卡控。
**方案**：CI 增加 `cargo-llvm-cov` / `cargo-tarpaulin` 步骤，低于阈值则 fail。
**ROI**：中（质量门控）。

## v1.0.0（远期）

- 多用户 / 多 ComfyUI 池（任务分发）
- Web UI（Tauri / egui）
- 提示词模板市场（在线下载社区主题）

## 已知技术债（不阻塞发布）

| 项 | 原因 | 处理时机 |
|----|------|---------|
| `prompt_engine::CombineContext` 不含 `project_root` | v0.1 重构后去除（combine 不需要路径） | 已处理 |
| `pipeline.rs` `submit_and_download` 收 `&Path` 而非 `&PathBuf` | clippy `ptr_arg` 警告修复 | 已处理 |
| cron 不支持 Quartz 扩展（`? L W #`） | 设计决策（避免过度复杂） | v0.2+ 按需 |
| `tags --lang` 必须在子命令前（`tags --lang zh list`） | clap 解析顺序 | 改用 `global = true` 可优化 UX |
