# 格式化规范

## 工具

- `rustfmt`（随 Rust 工具链安装）。
- CI 强制执行 `cargo fmt --check`。

## 配置

- 使用 `rustfmt.toml` 固定关键选项：
  - `max_width = 100`
  - `tab_spaces = 4`
  - `edition = "2021"`
  - `imports_granularity = "Crate"`

## 注释

- **公开 API 必须有文档注释**（`///`）。
- 注释说明**为什么**，不说明**做什么**（代码本身已表达）。
- 禁止行尾注释堆砌（超过 3 个连续 `// xxx` 视为应抽取为函数）。

## 错误信息

- 错误信息以小写字母开头，不以句号结尾。
- 包含足够的上下文（哪个模块、哪个参数），便于日志检索。

```rust
// ✅ 正确
Err(AppError::Config("missing key: comfyui.url".into()))

// ❌ 错误
Err(AppError::Config("Error!".into()))
```

## 字符串

- 用户可见字符串集中到 `src/i18n/zh_CN.rs`（仅在 CLI 输出场景）。
- 日志与错误信息使用英文（便于国际检索）。

## 行长度

- 软限制 100 字符。
- 字符串字面量不可拆分（即便超过）。
- 表格 / URL 不强制换行。