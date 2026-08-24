# CLI 设计

## 结论

CLI 采用 **clap 4.x derive 模式**，五个子命令覆盖所有使用场景：`generate` / `batch` / `daemon` / `tags` / `config`。子命令之间参数子集互不耦合，避免 `--batch 5 --daemon` 这类语义冲突。

## 命令清单

### `generate`（单次生成）

| 参数 | 短参 | 类型 | 默认 | 说明 |
|------|------|------|------|------|
| `--template` | `-t` | string | `default` | workflow 模板名 |
| `--tags` | | string[] | 自动 | 覆盖维度标签（key=value） |
| `--count` | `-n` | u32 | 1 | 生成张数（≤ 16） |
| `--refine` | | bool | true | 是否调用 LLM 优化 |
| `--output` | `-o` | path | 自动 | 图片输出目录 |
| `--seed` | | u64 | 随机 | 固定 seed 可复现 |
| `--lang` | `-l` | enum | `default_lang` | 输出语言：`zh` / `en` / `mixed` |

**示例**：
```bash
cargo run -- generate \
  --template portrait \
  --tags 发型=长发,首饰=项链 \
  --count 3 \
  --lang zh \
  --seed 42
```

### `batch`（一次性随机组合 N 张）

| 参数 | 短参 | 类型 | 默认 | 说明 |
|------|------|------|------|------|
| `--count` | `-n` | u32 | 10 | 随机组合张数（必填语义） |
| `--refine` | | bool | true | 是否 LLM 优化 |
| `--lang` | `-l` | enum | `default_lang` | 输出语言：`zh` / `en` / `mixed` |

**语义**：从 tags 各维度**随机抽取** N 组组合，每组生成 1 张图，串行执行后退出。**无并发参数**——并发属于调度层（`daemon`）的职责，不属于 batch 语义。

**与 `generate -n` 的差异**：`generate -n` 用同一组 tags 生成 N 张（仅 seed 不同）；`batch -n` 用 N 组不同的随机 tags 各生成 1 张。

### `daemon`（定时任务）

| 参数 | 短参 | 类型 | 默认 | 说明 |
|------|------|------|------|------|
| `--interval` | `-i` | duration | — | 固定间隔（如 `30m`） |
| `--cron` | | string | — | cron 表达式（与 interval 互斥） |
| `--at` | | datetime[] | — | 具体时刻列表（ISO8601） |
| `--mode` | `-m` | enum | — | `fixed`（固定 prompt）或 `auto`（自动随机组合 prompt） |
| `--prompt` | | string | — | `mode=fixed` 时必填：固定 prompt 文本 |
| `--prompt-file` | | path | — | `mode=fixed` 时可选：从文件加载 prompt（与 `--prompt` 互斥） |
| `--count-per-tick` | | u32 | 1 | 每次触发生成张数 |
| `--persist` | | path | `config/schedule.toml` | 任务持久化文件 |
| `--lang` | `-l` | enum | `default_lang` | 输出语言：`zh` / `en` / `mixed` |

**互斥规则**（clap `#[arg(conflicts_with)]` / `requires_if` 强制）：
- `--interval` / `--cron` / `--at` 三选一。
- `--prompt` 与 `--prompt-file` 互斥。
- `--mode = fixed` 必须同时提供 `--prompt` 或 `--prompt-file`。
- `--mode = auto` **禁止**提供 `--prompt` / `--prompt-file`。

**示例**：

```bash
# 固定 prompt：每小时生成同一段描述的不同变体
cargo run -- daemon --interval 1h --mode fixed --prompt "长发美女在海边"

# 自动 prompt：每小时随机组合 tags
cargo run -- daemon --interval 1h --mode auto

# 从文件加载 prompt
cargo run -- daemon --interval 1h --mode fixed --prompt-file ./prompts/character.md
```

### `tags`（标签管理）

| 子命令 | 说明 |
|--------|------|
| `tags list [--lang <lang>]` | 列出指定语言下的所有维度文件与每行计数（默认 `default_lang`） |
| `tags show <维度> [--lang <lang>]` | 显示某维度全部 tag |
| `tags add <维度> <tag> [--lang <lang>]` | 追加一条 tag（去重） |
| `tags remove <维度> <tag> [--lang <lang>]` | 删除一条 tag |

**示例**：
```bash
cargo run -- tags list --lang zh
cargo run -- tags show 发型 --lang en
cargo run -- tags add 发型 长发 --lang zh
```

### `config`（配置查看）

| 子命令 | 说明 |
|--------|------|
| `config show` | 显示合并后（default + local + env）的配置 |
| `config validate` | 校验配置文件语法与必填项 |

## 全局参数

| 参数 | 短参 | 说明 |
|------|------|------|
| `--config-dir` | `-c` | 自定义配置目录 |
| `--verbose` | `-v` | 日志级别（可重复：`-vvv` = debug） |
| `--quiet` | `-q` | 静默模式（仅 error） |
| `--no-color` | | 禁用 ANSI 颜色 |

## 输出协议

- 默认 stdout：进度 + 最终结果摘要。
- 详细日志 → `logs/<command>-<timestamp>.log`（tracing-subscriber 控制）。
- 失败时 exit code：0=成功，1=业务错误，2=配置错误，3=网络错误。

## 关键决策

### 决策 1：子命令式而非全局 flag

**为什么**：`generate --batch` 与 `batch` 语义不同，前者是"用同一组 tag 生成多张"，后者是"随机组合多张"。混用会让参数语义模糊。

### 决策 2：tags 用 `key=value` 列表传入

**为什么**：CLI 单行参数不可能携带"未指定维度的随机选择"，但允许用户**精确覆盖**。未指定维度由程序按 tags 文件随机抽取。

### 决策 3：daemon 的三种调度模式互斥

**为什么**：interval / cron / at-list 表达的是三种调度哲学，混用会让调度器状态空间爆炸。

## 反模式

- ❌ 一长串 `--key1 --key2 --key3` 全局 flag（应改子命令）。
- ❌ `count` 默认值巨大（如 100），掩盖误用。
- ❌ 进度信息混在 stderr 与 stdout（拆分明示）。