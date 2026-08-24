# 提交规范

## 格式

```
<type>(<scope>): <subject>

<body>

<footer>
```

### Type

| Type | 用途 |
|------|------|
| `feat` | 新功能 |
| `fix` | 缺陷修复 |
| `refactor` | 重构（不改变行为） |
| `docs` | 仅文档变更 |
| `test` | 仅测试变更 |
| `chore` | 构建 / 工具链 / 杂项 |
| `perf` | 性能优化 |

### Scope

本项目的约定 scope：`cli`、`config`、`comfyui`、`prompt`、`scheduler`、`deps`、`docs`。

### Subject

- 中文，≤ 30 字，祈使语气（"新增"而非"已新增"）。
- 不以句号结尾。

### Body

- 解释**为什么**做这个改动，而非做了什么。
- 与 `subject` 之间空一行。

### Footer

- 关联 Issue：`Refs #123` 或 `Closes #123`。
- 破坏性变更：`BREAKING CHANGE: <描述>`。

## 示例

```
feat(prompt): 新增 tags 多维度拼接能力

将 tags 目录下按维度划分的 txt 文件组合为单一 prompt 字符串。
拼接顺序遵循字典序，避免依赖文件系统读取顺序。

Refs #12
```

## 不允许

- ❌ 巨型提交（多个不相关改动）。
- ❌ 含糊的 subject（"fix bug"、"update code"）。
- ❌ 在 commit 中混入生成的文件（output/、target/）。