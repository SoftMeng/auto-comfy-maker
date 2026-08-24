# Workflow 模板替换机制

## 结论

ComfyUI workflow 是 **JSON 数据**，本项目把它当成模板：用 `serde_json::Value` 加载，按 **节点 ID + 字段路径** 替换指定字段（最常见的是 prompt 文本与 seed），然后原样 POST 给 ComfyUI。模板本身不进 Rust 代码、保持纯 JSON。

## 模板目录结构

```
templates/
├── default.json          # 默认 workflow（SDXL 基础）
├── portrait.json         # 人像专用
├── landscape.json        # 风景
├── product.json          # 商品图
└── MANIFEST.toml         # 节点 ID 与字段映射表
```

### MANIFEST.toml 示例

```toml
[templates.default]
positive_prompt_node = "6"
positive_prompt_field = "inputs.text"
negative_prompt_node = "7"
negative_prompt_field = "inputs.text"
seed_node = "3"
seed_field = "inputs.seed"

[templates.portrait]
positive_prompt_node = "10"
positive_prompt_field = "inputs.text"
negative_prompt_node = "11"
negative_prompt_field = "inputs.text"
seed_node = "5"
seed_field = "inputs.seed_no"
```

**为什么需要 MANIFEST**：ComfyUI 不同 workflow 的节点命名不一致（用户拖拽决定）。把映射外置到 toml 而非硬编码在 Rust，避免每次新增模板都改代码。

## 替换流程

```
加载模板 JSON ─┐
               ├─▶ serde_json::Value
加载 MANIFEST ─┘        │
                         ▼
              ┌──────────────────────┐
              │ WorkflowReplacer     │
              │ .replace(path, value) │
              └──────────┬───────────┘
                         │
                         ▼
              修改后的 Value（直接 POST）
```

### Rust 实现核心

```rust
pub struct WorkflowReplacer<'a> {
    value: &'a mut serde_json::Value,
}

impl<'a> WorkflowReplacer<'a> {
    pub fn replace(&mut self, field_path: &str, new_value: impl Into<serde_json::Value>) -> Result<(), ReplaceError> {
        let pointer = field_path_to_pointer(field_path);
        self.value.pointer_mut(&pointer)
            .ok_or(ReplaceError::PathNotFound(field_path.into()))?
            = new_value.into();
        Ok(())
    }
}
```

`field_path_to_pointer` 把 `inputs.text` 转为 `/inputs/text`，与 JSON Pointer 规范对齐。

## 调用顺序（典型）

```rust
// 1. 加载
let mut workflow: serde_json::Value = serde_json::from_str(&template_json)?;
let manifest = Manifest::load("templates/MANIFEST.toml")?;
let spec = manifest.get("portrait")?;

// 2. 替换
{
    let mut replacer = WorkflowReplacer { value: &mut workflow };
    replacer.replace(&spec.positive_prompt_field, prompt)?;
    replacer.replace(&spec.negative_prompt_field, negative_prompt)?;
    replacer.replace(&spec.seed_field, seed)?;
}

// 3. 提交
comfyui_client.submit(&workflow).await?;
```

## 关键决策

### 决策 1：节点 ID 映射在 MANIFEST 而非代码

**为什么**：用户新增 workflow 模板不应触发代码变更。把"哪里是 prompt"这种领域知识交给 toml。

### 决策 2：使用 JSON Pointer 而非手写路径解析

**为什么**：标准 RFC 6901 路径在 serde_json 中有原生支持（`pointer_mut`），减少自实现 bug。

### 决策 3：不缓存模板到内存

**为什么**：模板文件小（< 50 KB），加载开销可忽略；缓存会让"模板热更新"复杂化。

## 错误类型

```rust
#[derive(Debug, thiserror::Error)]
pub enum ReplaceError {
    #[error("path not found in workflow: {0}")]
    PathNotFound(String),
    #[error("path exists but value type mismatch: {0}")]
    TypeMismatch(String),
    #[error("manifest missing entry for template: {0}")]
    ManifestMissing(String),
}
```

## 反模式

- ❌ 在 Rust 代码中写 `let json = r#"{"6": {"inputs": {"text": ...}}}"#`。
- ❌ 模板 JSON 中含 `// prompt 注入点` 注释占位（依赖脆弱字符串匹配）。
- ❌ 用正则解析 JSON（应使用 serde_json 类型化操作）。