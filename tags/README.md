# tags 目录

本目录存储**元素（elements / tags）**——是组成 prompt 的**最小词元**，不是 prompt 本身。

## 关键概念

| 概念 | 是什么 | 在哪里 |
|------|--------|--------|
| **tag（元素）** | 单一词条，如"长发"、"项链"、"海边" | `tags/{lang}/{dim}.txt` |
| **theme（主题/配方）** | 元素的组织规则：哪些必选、哪些随机、哪些互斥、顺序如何 | `themes/{name}.toml` |
| **prompt（提示词）** | theme 引用 tags 元素，经引擎组合后的**最终文本** | 运行期生成 |

**关系**：`tags + theme → engine → prompt`

`tags/` 是数据，`themes/` 是规则，`prompt_engine` 是把两者组合的通用解释器（**零业务逻辑**，所有规则来自 theme）。

## 目录结构

```
tags/
├── zh/                       # 中文元素
│   ├── 发型.txt
│   ├── 首饰.txt
│   ├── 场景.txt
│   ├── 服装.txt
│   ├── 表情.txt
│   └── 构图.txt
└── en/                       # 英文元素（SDXL / Flux 友好）
    ├── hairstyle.txt
    ├── jewelry.txt
    ├── scene.txt
    ├── outfit.txt
    ├── expression.txt
    └── composition.txt
```

## 文件格式

每行一个 tag；空行忽略；以 `#` 开头视为注释。

```
# 中文示例（tags/zh/发型.txt）
长发
短发
卷发
盘发
```

## 元素分类（type）

每个维度文件可声明分类类型（写入文件名后缀或 toml 侧车文件，**本项目首版用文件后缀**）：

- `simple`：每个元素独立可选，无冲突。默认。
- `grouped`：元素分组，组内互斥（`{hair.short}.<file>.toml`）
- `optional`：以概率被选中（`{hair.optional}.<file>.toml`）
- `nested`：嵌套子分类（如 `clothing.top/bottom/shoes`）

**首版仅实现 `simple`**，其他类型在 theme 中描述。

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
cargo run -- generate --lang zh --theme portrait
```

英文用户 / 用 SDXL / Flux：

```bash
cargo run -- generate --lang en --theme portrait
```

### 混合模式（高级）

```bash
cargo run -- generate --lang mixed --theme portrait
```

**行为**：theme 同时引用 `zh` 与 `en` 元素；engine 按 theme 规则组合（多用于精细主题）。

## 默认语言

`config/default.toml` 中：

```toml
[prompt]
default_lang = "zh"    # 可选: zh | en | mixed
```

可通过环境变量 `AUTO_COMFY_DEFAULT_LANG` 覆盖。

## 添加新语言

1. 在 `tags/` 下新建 `<lang>/` 目录。
2. 在该目录下添加 `<dimension>.txt` 文件。
3. 无需改 Rust 代码（自动发现）。
4. CLI 使用 `--lang <lang>` 即可。

## 迁移指引（从旧版本）

旧版本把 `tags/` 内容直接当作"提示词"使用。迁移步骤：

- 把 `tags/发型.txt`（如果含完整句子）拆成最小词元（如"长发"、"短发"）。
- 在 `themes/<主题>.toml` 中声明如何使用这些词元。

## 错误示例（曾经犯的错）

```text
# ❌ 错误：tag 不应该是完整句子
tags/zh/角色.txt:
  一位长发美女在海边,穿着白裙,面带微笑,3D 渲染风格
```

```text
# ✅ 正确：tag 是最小词元
tags/zh/角色.txt:
  3D 渲染风格
  日系动漫风格
  写实风格
  油画风格
  赛博朋克风格
```

组合成完整 prompt 的工作由 **theme** 负责，而非由 tag 自身承担。