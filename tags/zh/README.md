# 中文 tags（zh）

本目录存储**中文 prompt 标签**。每个 `.txt` 文件对应一个维度。

## 文件命名

维度名直接用作文件名，使用中文：

- `发型.txt`
- `首饰.txt`
- `场景.txt`
- `服装.txt`
- `表情.txt`
- `构图.txt`

## 文件格式

每行一个 tag：

```
长发
短发
卷发
盘发
```

## 加载时机

当 `--lang zh` 或未指定（按 `[prompt].default_lang` 默认值）时加载。

## 添加新维度

1. 在本目录新建 `<维度>.txt`。
2. 在 `config/default.toml` 的 `[prompt].default_dimensions` 添加维度名。
3. 无需改 Rust 代码（自动发现）。

## 添加新 tag

CLI：
```bash
cargo run -- tags add 发型 短发
```
或直接编辑文件提交。