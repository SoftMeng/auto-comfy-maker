# 配置文件结构

## 结论

配置三层优先级：**`config/default.toml`**（仓库默认） → **`config/local.toml`**（本地覆盖，gitignore） → **环境变量 / CLI 参数**（运行时最高）。后层覆盖前层。配置加载后以强类型 `AppConfig` 暴露给业务代码，避免散落字段。

## 优先级链

```
环境变量 (COMFYUI_URL 等)
        ↓ 覆盖
CLI 参数 (--config-dir, --template 等)
        ↓ 覆盖
config/local.toml
        ↓ 覆盖
config/default.toml
        ↓
   AppConfig (强类型)
```

## 完整 default.toml

```toml
# 全局设置
[app]
name = "auto-comfy-maker"
log_level = "info"
log_dir = "logs"
output_dir = "output"

# ComfyUI 连接
[comfyui]
url = "http://127.0.0.1:8188"
timeout_secs = 300
poll_interval_ms = 1000
api_key = ""  # 若 ComfyUI 启用鉴权

# LLM（可选；由 rig-core 0.40 调度）
[llm]
enabled = false
provider = "openai"        # openai | anthropic（与 rig-core provider 枚举对齐）
model = "gpt-4o-mini"      # openai: gpt-4o-mini / gpt-4o；anthropic: claude-3-5-sonnet 等
api_key = ""               # 也可读环境变量 OPENAI_API_KEY / ANTHROPIC_API_KEY
base_url = "https://api.openai.com/v1"  # provider = openai 时生效
disable_thinking = false   # true 时向 provider 传 enable_thinking=false（DashScope/Qwen 推荐关闭）
max_tokens = 500
temperature = 0.7

# Prompt 拼接
[prompt]
strategy = "comma"           # comma | newline | natural
max_length = 800
default_lang = "zh"          # zh | en | mixed
default_dimensions = ["发型", "首饰", "场景", "服装"]

# 默认 ComfyUI 模板
[template]
default_name = "default"
templates_dir = "templates"
manifest_file = "MANIFEST.toml"

# 调度器默认值（仅当未指定 CLI 参数时使用）
[scheduler]
default_interval_secs = 1800
retry_max = 3
retry_backoff_secs = [0, 30, 120]
```

## 加载流程

```rust
pub fn load(config_dir: &Path) -> Result<AppConfig, ConfigError> {
    let mut config = AppConfig::default();

    // 1. 加载 default.toml
    let default_path = config_dir.join("default.toml");
    if default_path.exists() {
        config.merge(toml::from_str(&fs::read_to_string(&default_path)?)?);
    }

    // 2. 加载 local.toml（可选）
    let local_path = config_dir.join("local.toml");
    if local_path.exists() {
        config.merge(toml::from_str(&fs::read_to_string(&local_path)?)?);
    }

    // 3. 应用环境变量覆盖
    config.apply_env_overrides();

    // 4. 校验
    config.validate()?;

    Ok(config)
}
```

## 环境变量映射

| 变量 | 字段 | 说明 |
|------|------|------|
| `COMFYUI_URL` | `[comfyui].url` | ComfyUI 服务地址 |
| `COMFYUI_API_KEY` | `[comfyui].api_key` | 鉴权密钥 |
| `OPENAI_API_KEY` | `[llm].api_key` | OpenAI 密钥（provider=openai 时） |
| `OPENAI_BASE_URL` | `[llm].base_url` | 自定义 endpoint（provider=openai 时） |
| `ANTHROPIC_API_KEY` | `[llm].api_key` | Anthropic 密钥（provider=anthropic 时） |
| `AUTO_COMFY_LOG_LEVEL` | `[app].log_level` | 日志级别 |
| `AUTO_COMFY_CONFIG_DIR` | — | 配置目录（替代 CLI `--config-dir`） |
| `AUTO_COMFY_DEFAULT_LANG` | `[prompt].default_lang` | 默认输出语言 |

## 校验规则

| 字段 | 规则 |
|------|------|
| `[comfyui].url` | 必须是合法 URL（含 scheme） |
| `[llm].enabled = true` | 则 `[llm].api_key` 必须非空 |
| `[llm].provider` | 必须在 `{openai, anthropic}` 集合（与 rig-core provider 对齐） |
| `[prompt].strategy` | 必须在 `{comma, newline, natural}` 集合 |
| `[prompt].default_lang` | 必须在 `{zh, en, mixed}` 集合 |
| `[app].log_level` | 必须在 `{trace, debug, info, warn, error}` 集合 |
| `[scheduler].retry_max` | ≤ 10 |
| `[prompt].default_lang = "mixed"` | 必须同时配置 `[llm].enabled = true`（混合模式强制 LLM refine） |

## 关键决策

### 决策 1：分层加载而非单文件

**为什么**：开发者本地可能改 URL / API key，不应提交进仓库。三层覆盖让"仓库默认 + 个人覆盖"共存。

### 决策 2：环境变量覆盖 toml

**为什么**：容器化部署（Docker / k8s）几乎都用环境变量注入密钥；保留 toml 是给本地开发用。

### 决策 3：强类型 `AppConfig`

**为什么**：散落的 `&str` / `bool` 配置项会让业务代码到处 `unwrap()`。`AppConfig` 在加载阶段即校验完毕，业务代码拿到的是 `&Url` / `Duration` / 枚举。

### 决策 4：API key 既可写在 toml 也可写在 env

**为什么**：env 适合 CI / 容器；toml 适合本地开发（用 `local.toml` 隔离）。任一来源生效。

## 反模式

- ❌ 把 `api_key` 写进 `default.toml`（会进 git）。
- ❌ 用全局可变单例 `static CONFIG`（多测试并发互相污染）。
- ❌ 配置项变更不校验（运行时崩溃）。
- ❌ 配置路径硬编码 `"./config"`（应支持 `--config-dir`）。