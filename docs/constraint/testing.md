# 测试规范

## 层级

| 层级 | 位置 | 工具 | 速度目标 |
|------|------|------|----------|
| 单元测试 | `src/<module>.rs` 内 `#[cfg(test)]` | `cargo test` | < 1s/模块 |
| 集成测试 | `tests/*.rs` | `cargo test --test` | < 5s/文件 |
| 端到端 | `tests/e2e/`（可选） | 本地或 CI 手动 | < 60s |

## 命名

- 测试函数：`test_<行为描述>_<场景>`。
- 示例：`test_combine_tags_returns_unique_set`、`test_combine_tags_preserves_order`。

## 覆盖率

- 核心业务模块（`prompt_engine`、`comfyui`、`scheduler`）：**行覆盖 ≥ 70%**。
- 配置加载、CLI 解析：**行覆盖 ≥ 50%**。
- 由 `cargo tarpaulin` 或 `cargo-llvm-cov` 报告。

## 外部依赖

- HTTP 请求必须 mock（`mockito` 或 `wiremock`）。
- 文件系统操作使用 `tempfile`。
- 禁止在测试中连接真实 ComfyUI 或 OpenAI 服务。

## 不变量测试

- 对纯函数（`combine`、`refine`、`replace_workflow_node`）覆盖：
  - 空输入
  - 单元素输入
  - 大量输入（性能基线）
  - 错误输入（解析失败）

## 反模式

- ❌ 测试只覆盖 happy path。
- ❌ `#[ignore]` 长期堆积（应修复或删除）。
- ❌ 共享可变状态（每个测试自己构造 fixture）。