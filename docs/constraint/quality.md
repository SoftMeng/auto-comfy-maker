# 质量门控

## CI 检查项（必须全部通过）

| 检查 | 命令 | 阻断 |
|------|------|------|
| 格式化 | `cargo fmt --check` | 是 |
| Lint | `cargo clippy --all-targets -- -D warnings` | 是 |
| 测试 | `cargo test --all-features` | 是 |
| 构建 | `cargo build --release` | 是 |
| 文档 | `cargo doc --no-deps` | 是 |
| 审计 | `cargo audit` | 警告 |

## 预提交 Hook

`scripts/pre-commit.sh`（建议）：

```bash
#!/usr/bin/env bash
set -e
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test --lib
```

## PR 模板检查项

- [ ] 关联 Issue
- [ ] 描述"为什么"而非"做了什么"
- [ ] 测试已新增 / 更新
- [ ] 文档（CLAUDE.md / docs/）已同步
- [ ] 无生成文件入库
- [ ] 无敏感信息（API key、个人 token）

## 性能基线

- 单次生成（含 ComfyUI 调用）端到端 ≤ 30s（视硬件）。
- tags 拼接 ≤ 10ms（100 维度 × 100 标签）。
- 调度器空转 CPU < 1%。

## 已知技术债登记

- 见 `docs/ROADMAP.md`（不在 CLAUDE.md 中维护）。