# 命名规范（Rust）

## 核心规则

| 类型 | 规范 | 示例 |
|------|------|------|
| 模块 / 文件 | snake_case | `prompt_engine.rs` |
| 函数 / 变量 | snake_case | `combine_tags()` |
| 类型 / Trait | PascalCase | `PromptEngine` |
| 常量 | SCREAMING_SNAKE_CASE | `MAX_BATCH_SIZE` |
| 枚举变体 | PascalCase | `ScheduleKind::Cron` |
| 错误类型后缀 | `Error` | `ComfyuiError` |
| 特化转换 | `From<T>` | `impl From<io::Error> for AppError` |

## 生命周期参数

- 使用描述性名称：`'a`（短）、`'config`（语义化）。
- 避免出现 `'static`，除非确实需要。

## crate 内命名

- 二进制 crate：`auto-comfy-maker`。
- 库 crate（若拆出）：`comfy_maker_core`。
- 特性（features）：`llm-refine`、`daemon`、`tracing`。

## tags 文件命名

- 维度名使用小写 + 中划线或下划线均可，但**项目内统一**。
- 推荐：`发型.txt`、`首饰.txt`、`场景.txt`（与业务语义对齐）。
- 不允许：`HairstyleTags.txt`、`hair_style_2.txt`（混用大小写或带数字版本号）。

## 配置文件命名

- `default.toml`：仓库默认配置。
- `local.toml`：开发者本地覆盖。
- `schedule.toml`：定时任务持久化（运行时生成）。

## 反模式

- ❌ 用 `Manager`、`Helper`、`Util` 作模块名（除非职责单一）。
- ❌ 缩写：`cfg`、`ctx`、`mgr` 在公共 API 中应展开。
- ❌ 双重否定命名：`disable_optimization: bool`（应使用 `optimize: bool`）。