# tags 目录

本目录存储**多语言、按维度划分**的 prompt 标签。语言是**一级目录**，维度是**二级目录**。

## 目录结构

```
tags/
├── zh/                       # 中文 tags
│   ├── 发型.txt
│   ├── 首饰.txt
│   ├── 场景.txt
│   ├── 服装.txt
│   ├── 表情.txt
│   └── 构图.txt
└── en/                       # 英文 tags（Stable Diffusion / SDXL / Flux 友好）
    ├── hairstyle.txt
    ├── jewelry.txt
    ├── scene.txt
    ├── outfit.txt
    ├── expression.txt
    └── composition.txt
```

## 为什么按语言切目录

- **物理隔离**：中英文是两种独立的词汇表，物理分层让意图自现。
- **零代码扩展**：新增 `ja/` `ko/` 等只需新建目录。
- **git diff 清晰**：跨语言改动不互相干扰，便于多人协作。

## 文件格式

每行一个 tag；空行忽略；以 `#` 开头视为注释。

```
# 中文示例（tags/zh/发型.txt）
长发
短发
卷发
盘发
```

```
# English example (tags/en/hairstyle.txt)
long hair
short hair
curly hair
updo
```

## 加载机制

`TagStore::load(lang)` 实例化对应语言的标签集，加载到 `IndexSet<String>`（自动去重）。

| 触发条件 | 加载路径 |
|---------|---------|
| `--lang zh`（或未指定 + `default_lang = "zh"`） | `tags/zh/` |
| `--lang en` | `tags/en/` |
| `--lang mixed` | 同时加载 `zh` 与 `en`，阶段一按维度拼接、阶段二 LLM 统一润色 |

## 选择策略

### 单语言（推荐）

中文用户：

```bash
cargo run -- generate --lang zh
```

英文用户 / 用 SDXL / Flux：

```bash
cargo run -- generate --lang en
```

### 混合模式（高级）

```bash
cargo run -- generate --lang mixed
```

**行为**：阶段一 `combine()` 把中英文 tags **按维度拼接**（同一维度内 zh tag 在前、en tag 在后），阶段二 `refine()` 调用 LLM 重新组织为自然语言。

**风险**：LLM 可能丢失部分维度。**仅在配置 LLM 时使用**。

## 默认语言

`config/default.toml` 中：

```toml
[prompt]
default_lang = "zh"    # 可选: zh | en | mixed
```

可通过环境变量 `AUTO_COMFY_DEFAULT_LANG` 覆盖。

## 添加新语言

1. 在 `tags/` 下新建 `<lang>/` 目录。
3. 在该目录下添加 `<dimension>.txt` 文件。
2. 无需改 Rust 代码（自动发现）。
3. CLI 使用 `--lang <lang>` 即可。

## 迁移指引（从旧版本）

旧版本使用单层目录（如 `tags/发型.txt`）。迁移步骤：

```bash
mkdir -p tags/zh
git mv tags/发型.txt tags/zh/
git mv tags/首饰.txt tags/zh/
# ... 其他维度
```

如有英文版（`tags/hairstyle.txt`），迁移到 `tags/en/`。