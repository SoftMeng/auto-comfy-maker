# Prompt 两阶段生成

## 结论

prompt 生成是**两阶段流水线**：阶段一 `combine()` 是确定性纯函数（多维度 tag 拼接）；阶段二 `refine()` 是可选的 LLM 优化。两者解耦，`refine()` 失败时必须**回退到阶段一结果**，而非整体任务失败。

## 阶段一：combine（确定性拼接）

### 输入

```rust
pub struct CombineContext {
    pub lang: Lang,                          // 输出语言（zh / en / mixed）
    pub dimensions: Vec<DimensionSelection>, // 用户选定的维度
    pub random_pool: LangAwarePool,          // 按语言组织的可选维度
    pub strategy: CombineStrategy,
    pub max_length: usize, // 字符上限，避免 ComfyUI 截断
}

pub enum Lang {
    Zh,
    En,
    Mixed, // 中英同维拼接，依赖 LLM refine 兜底
}

pub struct LangAwarePool {
    pub zh: HashMap<String, Vec<String>>,
    pub en: HashMap<String, Vec<String>>,
}

pub enum DimensionSelection {
    Fixed { name: String, value: String },   // 用户指定（值需与 lang 匹配）
    Random { name: String },                  // 从 lang 对应池随机抽
    Skipped(String),                          // 跳过
}

pub enum CombineStrategy {
    Comma,       // 英文逗号 + 空格
    Newline,     // 换行（适合结构化 prompt）
    Natural,     // 英文句式（"with long hair, wearing necklace"）
}
```

### 输出

`String`：拼接后的 prompt 文本。

### 算法

```
1. 根据 ctx.lang 选定主池（zh / en / mixed）
2. 遍历 dimensions
3. 对每个维度：
   - Fixed → 直接取 value（Mixed 时要求 value 同时含中英）
   - Random → 从对应池随机抽一条
     · Mixed 时：先抽 zh 再抽 en，二者顺序拼接
   - Skipped → 忽略
4. 按 strategy 拼接
5. 若超过 max_length，按 strategy 的优先级截断（先丢 random 维度）
6. 返回
```

### 语言行为表

| Lang | 加载路径 | 输出策略 | LLM refine 要求 |
|------|---------|---------|----------------|
| `Zh` | `tags/zh/` | 仅中文 tag 拼接 | 可选 |
| `En` | `tags/en/` | 仅英文 tag 拼接 | 可选 |
| `Mixed` | `tags/zh/` + `tags/en/` | 中英同维拼接（zh tag 在前、en tag 在后） | **强制**（无 LLM 则回退到 zh） |

**为什么 Mixed 强制 LLM**：中英拼接后是机械拼接，自然度差；没有 LLM refine 时反而不如单语言。`refine()` 失败时直接回退到 zh 单语言结果（而非混合输出）。

### 纯函数保证

- 不读取全局状态（除 `random_pool`）。
- 不发起网络请求。
- 同输入必同输出（除 Random 维度）。

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