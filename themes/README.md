# themes 目录

本目录存储 **主题/配方（theme）**——把 `tags/` 中的元素组织成完整 prompt 的**声明式规则**。

## 概念边界

| 概念 | 是什么 | 在哪里 |
|------|--------|--------|
| **tag** | 最小词元（"长发"、"项链"） | `tags/{lang}/{dim}.txt` |
| **theme** | 元素的组织规则（必选/随机/互斥/顺序/嵌套） | `themes/{name}.toml` |
| **prompt** | theme + tags 经引擎组合后的最终文本 | 运行期生成 |

**核心原则**：`prompt_engine` 是**通用解释器，零业务逻辑**。所有"如何组合"的规则都在 theme 文件中声明。

## 目录结构

```
themes/
├── portrait.toml         # 人像主题
├── landscape.toml        # 风景主题
├── product.toml          # 商品图主题
├── anime-character.toml  # 动漫角色
└── README.md             # 本文件
```

## 文件格式（TOML）

### 完整示例：`portrait.toml`

```toml
# ============================================================
# 元信息
# ============================================================
[meta]
id = "portrait"
name = "人像主题"
description = "适合单人物人像生成"
lang = "zh"               # 主语言：zh | en | mixed
version = "1.0"

# ============================================================
# 必选元素（按固定顺序，出现在 prompt 开头）
# ============================================================
[order.fixed]
# 格式：[category] = { file = "tags/zh/...txt", count = N }
style      = { file = "tags/zh/风格.txt",     count = 1 }
subject    = { file = "tags/zh/主体.txt",     count = 1 }

# ============================================================
# 随机元素（按主题声明的优先级，部分类目保持顺序）
# ============================================================
[order.random]
# 高优先级：保持顺序（如发型/镜头）
hair       = { file = "tags/zh/发型.txt",     count = 1 }
camera     = { file = "tags/zh/构图.txt",     count = 1 }

# 其他随机维度（先随机打乱类目，再选元素）
clothing   = { file = "tags/zh/服装.txt",     count = 1, max = 2 }
jewelry    = { file = "tags/zh/首饰.txt",     count = 0, max = 2 }
expression = { file = "tags/zh/表情.txt",     count = 1 }

# ============================================================
# 可选元素（按概率出现）
# ============================================================
[order.optional]
scene      = { file = "tags/zh/场景.txt", probability = 0.7 }
lighting   = { file = "tags/zh/光线.txt", probability = 0.5 }

# ============================================================
# 冲突规则
# ============================================================
# 同一组内的元素不能同时出现
[compatibility.conflicts]
clothing = [
  ["比基尼", "毛衣"],      # 季节冲突
  ["汉服", "牛仔裤"],      # 时代冲突
]
jewelry = [
  ["金项链", "银项链"],    # 风格冲突
]

# ============================================================
# 生成选项
# ============================================================
[generation]
max_elements = 30        # prompt 最多元素数
max_length = 800         # 字符上限
```

## 字段含义

### `[meta]`

| 字段 | 必填 | 说明 |
|------|------|------|
| `id` | ✅ | 主题唯一 ID，CLI `--theme <id>` 使用 |
| `name` | ✅ | 可读名称 |
| `description` | — | 描述 |
| `lang` | ✅ | 主语言：决定加载 `tags/zh/` 还是 `tags/en/` 或两者 |
| `version` | — | 主题版本（用于未来兼容） |

### `[order.fixed]` / `[order.random]` / `[order.optional]`

| 段 | 行为 |
|----|------|
| `fixed` | 必选，**固定顺序**出现在 prompt 开头 |
| `random` | 必选，**类目按声明顺序**（部分类目可在类内随机） |
| `optional` | **按 probability 概率**出现 |

每个类目的字段：

| 字段 | 必填 | 说明 |
|------|------|------|
| `file` | ✅ | 引用的 tags 文件路径 |
| `count` | ✅ | 至少选几个元素 |
| `max` | — | 最多选几个（默认与 count 相同） |
| `probability` | 仅 optional | 0.0 - 1.0 |

### `[compatibility.conflicts]`

组内元素互斥。例：`clothing = [["比基尼", "毛衣"]]` 表示同一主题中这两个元素不能同时被选中。

## CLI 使用

```bash
# 使用指定主题
cargo run -- generate --theme portrait --lang zh

# batch 模式（每个生成用同一主题）
cargo run -- batch -n 5 --theme portrait --lang en

# daemon auto 模式（每个 tick 用同一主题随机生成）
cargo run -- daemon --interval 1h --mode auto --theme portrait

# daemon fixed 模式（忽略 theme，使用 --prompt）
cargo run -- daemon --interval 1h --mode fixed --prompt "..."
```

## 引擎视角

`prompt_engine` 加载 theme 后按以下流程生成 prompt：

```
1. load theme.toml
2. load tags/{lang}/* 涉及的所有文件
3. 按 order.fixed 顺序从每个类目取 count 个元素
4. 按 order.random 顺序从每个类目取 count-max 个元素
5. 验证 [compatibility.conflicts]，回溯替换冲突元素
6. 按 probability 决定 optional 类目是否参与
7. 用策略（comma/newline/natural）拼接
8. 若超 max_length，按优先级截断
9. 输出最终 prompt
```

**关键**：`prompt_engine` 不写死任何业务规则——加新主题只需新增 `themes/{name}.toml`，**无需改 Rust 代码**。

## 添加新主题步骤

1. 在本目录新建 `<name>.toml`。
2. 填写 `[meta]` 与 `[order.*]` 段。
3. 确保 `tags/{lang}/` 下有引用的所有文件。
4. 跑 `cargo run -- generate --theme <name> --lang <lang>` 验证。
5. 提交 PR 时附 1-2 张示例生成图。

## 参考实现

设计参考 glmclaw 的 `src/prompts/themes/*.ts`：
- theme 声明式配置（`meta` / `elements` / `order` / `compatibility`）
- engine 通用解释器（`engine.ts` 的 `generate()` 函数）
- 业务逻辑完全在 theme 中，engine 零业务

我们用 toml 替代 .ts，相同思想。