# 目录结构与模块职责

## 模块边界

| 模块 | 单一职责 | 允许依赖 |
|------|----------|----------|
| `cli` | 解析命令行参数、调度子命令 | config, scheduler, prompt_engine, comfyui |
| `config` | 加载 / 校验配置文件 | 无业务依赖 |
| `comfyui` | HTTP 客户端、workflow 提交与轮询 | config |
| `prompt_engine` | 标签组合 + LLM 优化 | config, comfyui（仅读取 prompt 文本） |
| `scheduler` | cron 解析、周期循环、持久化 | config, prompt_engine, comfyui |
| `main.rs` | 组装依赖、初始化日志 | 所有模块 |

**禁止**：
- `config` 依赖任何业务模块（避免循环）。
- `comfyui` 依赖 `prompt_engine`（HTTP 层不应理解业务）。
- `cli` 直接构造 HTTP 请求（必须通过 `comfyui`）。

## 运行时目录

| 目录 | 提交 | 说明 |
|------|------|------|
| `templates/` | ✅ | ComfyUI workflow JSON 模板 |
| `tags/` | ✅ | 多维度标签文件 |
| `config/default.toml` | ✅ | 默认配置 |
| `config/local.toml` | ❌ | 本地覆盖 |
| `config/schedule.toml` | ❌ | 定时任务持久化 |
| `output/` | ❌ | 生成的图片 |
| `logs/` | ❌ | 运行日志 |

`.gitignore` 必须包含：`config/local.toml`、`output/`、`logs/`、`config/schedule.toml`、`target/`。

## 路径解析

- 所有路径相对于项目根（`CARGO_MANIFEST_DIR`），不依赖 `std::env::current_dir()`。
- 用户可通过 `--config-dir <path>` 覆盖默认目录。

## 测试目录

- 单元测试：`src/<module>.rs` 内 `#[cfg(test)] mod tests`。
- 集成测试：`tests/<feature>.rs`。
- Fixtures：`tests/fixtures/`（小体积示例数据）。
- 不允许在源码目录创建 `test_data/` 或 `__test__/`。