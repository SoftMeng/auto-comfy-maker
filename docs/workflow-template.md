# Workflow 模板替换机制

## 结论

ComfyUI workflow 是 **JSON 数据**，本项目把它当成模板：用 `${positive_prompt}` / `${seed}` / `${width}` / `${height}`（及中文别名 `${提示词}` 等）做**字符串占位符替换**，然后原样 POST 给 ComfyUI。节点 ID 完全无关——ComfyUI 导出时怎么编号都无所谓。

## 模板目录结构

```
templates/
├── zimage.json           # Z-Image 模型（来自 glmclaw）—— ${...} 占位符模式
├── anima.json            # Anima 模型（来自 traceclaw）—— ${...} 占位符模式
├── anima-lora.json       # Anima + LoRA 工作流（用户上传）—— ${...} 占位符模式
└── <your_workflow>.json  # 你自己从 ComfyUI 导出的（用占位符替换注入点）
```

加新 workflow：

1. ComfyUI UI 搭好 workflow，把想注入的字段写成 `${positive_prompt}` / `${seed}` / `${width}` / `${height}`，Save (API Format) 导出 JSON
2. 复制到 `templates/<name>.json`
3. 跑 `cargo run -- generate --template <name>`
4. **0 行代码改动**

## 占位符清单

| 占位符 | 类型 | 含义 |
|--------|------|------|
| `${positive_prompt}` 或 `${提示词}` | 字符串 | positive prompt 内容 |
| `${seed}` 或 `${种子}` | 整数 | 随机种子（默认从时间戳生成） |
| `${width}` 或 `${宽}` | 整数 | 图片宽度（缺省 768） |
| `${height}` 或 `${高}` | 整数 | 图片高度（缺省 1536） |

英文 + 中文别名同时支持。模板里写哪种都行。

## 当前内置模板

## 替换流程

```
读 templates/<name>.json
        │
        ▼
substitute(text, prompt, seed, width?, height?)
  字符串替换 str::replace
        │
        ▼
serde_json::from_str(&substituted)
  验证 JSON 合法
        │
        ▼
POST /prompt 给 ComfyUI
```

## Rust 实现核心

```rust
pub fn substitute(
    template: &str,
    positive_prompt: &str,
    seed: i64,
    width: Option<i64>,
    height: Option<i64>,
) -> String {
    let mut out = template
        .replace("${positive_prompt}", positive_prompt)
        .replace("${提示词}", positive_prompt);
    let seed_str = seed.to_string();
    out = out
        .replace("${seed}", &seed_str)
        .replace("${种子}", &seed_str);
    if let Some(w) = width {
        let ws = w.to_string();
        out = out.replace("${width}", &ws).replace("${宽}", &ws);
    }
    if let Some(h) = height {
        let hs = h.to_string();
        out = out.replace("${height}", &hs).replace("${高}", &hs);
    }
    out
}
```

纯字符串替换 + 节点 ID 完全无关——这是与 glmclaw `buildWorkflow` 同款做法。

## 关键决策

### 决策 1：用占位符而非 JSON Pointer / MANIFEST

**为什么**：用户从 ComfyUI 导出 workflow 时节点 ID 是系统自动生成的；用占位符让"加新 workflow 不需要懂节点 ID"。
**放弃**：之前实验过的 MANIFEST + JSON Pointer 方案（已废弃，git 历史可查）。

### 决策 2：占位符与 JSON 字面量共存

模板里 `${positive_prompt}` 是**纯字符串片段**——出现在 `"text": "${positive_prompt}"` 中。substitute 后整个 JSON 仍是合法 JSON。

**唯一约束**：模板本身必须是合法 JSON（含占位符的版本），所以字符串值必须有引号包围。如 `"text": "${提示词}"` 正确，`"text": ${提示词}` 错误。

### 决策 3：diemnsion 占位符是 Optional

`--width --height` 不传时，模板里的 `${width}` / `${height}` 字面量**保留**——给某些 workflow 完全用硬编码尺寸留余地。

## 反模式

- ❌ 在 Rust 代码里硬编码节点 ID（违反"加 workflow 不动代码"原则）
- ❌ 用 `serde_json::Value` 的 `pointer_mut` 做字段替换（已被替代方案替换）
- ❌ 在模板里用 `// prompt 注入点` 注释占位（依赖脆弱字符串匹配）

## 真实模板示例（zimage.json 节选）

```json
{
  "91": {
    "inputs": {
      "text": "${提示词}, stunning composition,",
      "clip": ["62", 0]
    },
    "class_type": "Text Multiline"
  },
  "82": {
    "inputs": {
      "seed": "${种子}"
    },
    "class_type": "easy seed"
  },
  "68": {
    "inputs": {
      "width": "${宽}",
      "height": "${高}",
      "batch_size": 1
    },
    "class_type": "EmptySD3LatentImage"
  }
}
```

调用：
```bash
cargo run -- generate --theme portrait --lang zh --template zimage --width 768 --height 1536
```
