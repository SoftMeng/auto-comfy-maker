# ComfyUI Workflow 模板

本目录存放 ComfyUI workflow JSON 文件。每个 `.json` 是完整的 workflow（导出自 ComfyUI）。

## 命名约定

- `<purpose>.json`：用途命名（`portrait`、`landscape`、`product`）。
- `default.json`：回退默认模板。

## 节点映射

每个模板必须在 `MANIFEST.toml` 中登记节点 ID 与字段路径。详见 `docs/workflow-template.md`。

## 加载流程

1. 用户通过 CLI `--template <name>` 指定。
2. 项目读取 `templates/<name>.json`。
3. 项目读取 `templates/MANIFEST.toml` 找到对应节点映射。
4. 替换 prompt / seed 字段后 POST 给 ComfyUI。

## 当前模板

（待添加。）

## 添加新模板步骤

1. 在 ComfyUI 中导出 workflow JSON 到本目录。
2. 在 `MANIFEST.toml` 添加 `[templates.<name>]` 段。
3. 在 `docs/workflow-template.md` 标注节点 ID 来源。
4. 提交 PR 时附 1 张示例生成图。