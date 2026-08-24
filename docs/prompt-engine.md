# Prompt 生成：从 tags 到 prompt

## 结论

**tags ≠ prompt**。本项目的概念边界严格分离：

| 概念 | 是什么 | 在哪里 |
|------|--------|--------|
| **tag** | 元素（最小词元） | `tags/{lang}/{dim}.txt` |
| **theme** | 元素的组织规则 | `themes/{name}.toml` |
| **prompt** | theme + tags 经引擎组合的**最终文本** | 运行期生成 |

prompt 生成是**两阶段流水线**：
- 阶段一 `combine(theme, tags)` 是确定性纯函数（按 theme 规则从 tags 选元素并组合）。
- 阶段二 `refine(prompt)` 是可选的 LLM 优化。

两者解耦，`refine()` 失败时必须**回退到阶段一结果**，而非整体任务失败。

**关键原则**：`prompt_engine` 是**通用解释器，零业务逻辑**——所有"如何组合"的规则都来自 theme 文件。

## 阶段一：combine（确定性拼接）

### 输入

```rust
pub struct CombineContext {
    pub theme: Theme,                       // 主题/配方：定义哪些类目必选/随机/可选/冲突
    pub tags: LangAwarePool,                // 元素池：tags/{lang}/* 加载的结果
    pub strategy: CombineStrategy,          // 拼接策略
    pub max_length: usize,                  // 字符上限
    pub seed: u64,                          // 随机种子（可复现）
}

pub struct Theme {
    pub meta: ThemeMeta,                    // id / name / lang / version
    pub order_fixed: Vec<CategoryRef>,      // 必选 + 固定顺序
    pub order_random: Vec<CategoryRef>,     // 必选 + 类目按声明顺序
    pub order_optional: Vec<(CategoryRef, f32)>, // 按 probability 可选
    pub conflicts: HashMap<String, Vec<Vec<String>>>, // 类目 → 互斥元素组
    pub generation: GenerationOptions,      // max_elements / max_length
}

pub struct CategoryRef {
    pub category: String,                   // 类目名（发型 / 首饰 / 服装 / 场景 / ...）
    pub file: PathBuf,                      // 对应 tags/{lang}/{file}.txt
    pub count: usize,                       // 最少选几个
    pub max: usize,                         // 最多选几个
}

pub struct LangAwarePool {
    pub zh: HashMap<String, Vec<String>>,   // 维度名 → 元素列表
    pub en: HashMap<String, Vec<String>>,
}

pub enum Lang {
    Zh,
    En,
    Mixed,
}

pub enum CombineStrategy {
    Comma,       // 英文逗号 + 空格
    Newline,     // 换行（适合结构化 prompt）
    Natural,     // 英文句式
}
```

### 输出

`String`：拼接后的 prompt 文本。

### 算法

```
1. 加载 theme，按 theme.meta.lang 选定主池（zh / en / mixed）
2. 遍历 theme.order_fixed：
   - 从对应池取 count 个元素（保持顺序）
3. 遍历 theme.order_random：
   - 高优先级类目（hair / camera）按声明顺序
   - 其他类目先随机打乱
   - 每个类目取 [count, max] 个元素
4. 验证 theme.compatibility.conflicts：
   - 冲突时回溯替换该元素（不重选整个类目）
5. 遍历 theme.order_optional：
   - 按 probability 决定是否参与；参与则取 count 个
6. 按 strategy 拼接
7. 若超过 max_length，按优先级截断：
   - 先丢 optional 元素
   - 再丢 random 元素
   - 最后丢 fixed 元素（保留至少 1）
8. 返回最终 prompt
```

### 语言行为表

| theme.meta.lang | 加载路径 | 输出策略 | LLM refine 要求 |
|----------------|---------|---------|----------------|
| `zh` | `tags/zh/` | 仅中文元素组合 | 可选 |
| `en` | `tags/en/` | 仅英文元素组合 | 可选 |
| `mixed` | `tags/zh/` + `tags/en/` | 同 theme 类目内 zh 在前 en 在后拼接 | **强制**（无 LLM 则回退到 zh） |

**为什么 Mixed 强制 LLM**：中英拼接后是机械拼接，自然度差；没有 LLM refine 时反而不如单语言。`refine()` 失败时直接回退到 zh 单语言结果（而非混合输出）。

### 纯函数保证

- 不读取全局状态（除传入的 `theme` / `tags`）。
- 不发起网络请求。
- 同输入（含相同 `seed`）必同输出。

## 阶段二：refine（LLM 优化）

### 输入

`combine()` 的输出字符串。

### 输出

`String`：LLM 优化后的 prompt。

### 算法

```
1. 构造 system prompt：告知 LLM 角色（"你是 Stable Diffusion prompt 专家"）
2. 构造 user prompt：附带 combine 输出 + 优化要求（"保留所有维度语义、提升细节与画面感"）
3. 调用 OpenAI Chat Completions API（gpt-4o-mini 或自选模型）
4. 返回 choices[0].message.content
```

### 回退策略

```
if refine().is_err():
    log::warn!("LLM refine failed, falling back to combine output");
    return combine_result;
```

**为什么必须回退**：LLM 调用可能因网络、配额、限流失败。图片生成的核心价值在于"最终能产图"，prompt 优化是锦上添花。

## 关键决策

### 决策 1：两阶段解耦为 trait

`PromptEngine` trait 定义见下方"LLM 实现"章节的 `DefaultPromptEngine` 代码块。

### 决策 2：combine 是同步函数，refine 是异步

**为什么**：combine 无 I/O，同步利于测试与性能；refine 必须异步（网络调用）。

### 决策 3：max_length 默认 800

**为什么**：SDXL 推荐 prompt 长度 ≤ 77 token；中文混合英文约 800 字符。超过即截断 random 维度，保留 fixed。

## 主题流程

```
┌────────────┐     ┌────────────┐     ┌────────────┐
│ tags/{lang}│     │ themes/    │     │            │
│ /*txt      │ ──▶ │ {name}.toml│ ──▶ │  combine() │
│ (元素数据)  │     │ (组合规则)  │     │            │
└────────────┘     └────────────┘     └─────┬──────┘
                                            │ 初步 prompt
                                            ▼
                                      ┌────────────┐
                                      │  refine()  │ (可选，LLM)
                                      └─────┬──────┘
                                            │  最终 prompt
                                            ▼
                                      ComfyUI
```

**清晰的责任划分**：
- **tags 目录**：只放"什么元素可用"，不知道如何组合。
- **themes 目录**：只放"如何组合"，不重复元素内容。
- **combine()**：通用解释器，按 theme 规则从 tags 池中选取元素。

加新主题 = 新增 `themes/<name>.toml` + 准备 `tags/{lang}/*` 文件，**无需改 Rust 代码**。

## tags 文件结构（按语言切目录）

```
tags/
├── zh/                      # 中文 tag 集合（自然语言表达友好）
│   ├── 发型.txt
│   ├── 首饰.txt
│   ├── 场景.txt
│   ├── 服装.txt
│   ├── 表情.txt
│   └── 构图.txt
└── en/                      # 英文 tag 集合（SDXL / Flux / Stable Diffusion 友好）
    ├── hairstyle.txt
    ├── jewelry.txt
    ├── scene.txt
    ├── outfit.txt
    ├── expression.txt
    └── composition.txt
```

- **同一语言内**：每行一条 tag，空行与 `#` 开头注释行忽略。
- **跨语言**：维度名可以不同（如 `发型` ↔ `hairstyle`），按 `tags/{lang}/{dim}.txt` 路径独立维护。
- **加载策略**：`TagStore::load(lang)` 按语言加载；切换语言需重新加载。

## LLM 实现：rig-core 0.40

`refine()` 阶段由 **rig-core 0.40** 实现，**不**自实现 OpenAI / Anthropic 客户端。

### Provider 抽象

```rust
pub enum ProviderKind {
    OpenAI(rig_core::providers::openai::CompletionsClient),
    Anthropic(rig_core::providers::anthropic::Client),
}

pub enum AgentKind {
    OpenAI(rig_core::agent::Agent<rig_core::providers::openai::CompletionModel>),
    Anthropic(rig_core::agent::Agent<rig_core::providers::anthropic::CompletionModel>),
}
```

直接采用 `agent_cli/src/agent/{builder.rs, provider.rs}` 的枚举分发模式，避免重复实现 provider 选择逻辑。

### PromptEngine 实现

```rust
#[async_trait]
pub trait PromptEngine: Send + Sync {
    fn combine(&self, ctx: &CombineContext) -> Result<String, PromptError>;
    async fn refine(&self, prompt: &str) -> Result<String, PromptError>;
}

pub struct DefaultPromptEngine {
    tags: TagStore,
    agent: Option<AgentKind>,     // None 时 refine 退化为 identity
}

impl PromptEngine for DefaultPromptEngine {
    async fn refine(&self, prompt: &str) -> Result<String, PromptError> {
        match &self.agent {
            AgentKind::OpenAI(a) => a.prompt(prompt).await
                .map_err(PromptError::from),
            AgentKind::Anthropic(a) => a.prompt(prompt).await
                .map_err(PromptError::from),
        }
    }
}
```

### 边界

- **rig 仅用于 LLM**；ComfyUI REST / WebSocket 仍由 reqwest 处理（`comfyui/` 模块）。
- **Provider 选择由 config 决定**：`[llm].provider = "openai" | "anthropic"`（见 `docs/config.md`）。

## 反模式

- ❌ 把 LLM 调用内嵌在 combine 里（混合同步与异步）。
- ❌ combine 内部读取全局文件系统状态（应通过 ctx 注入）。
- ❌ refine 失败时整体任务失败（违背"LLM 锦上添花"原则）。
- ❌ 把拼接策略硬编码在字符串里（应通过枚举 + 模式匹配）。