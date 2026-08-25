# 定时任务与周期

## 结论

调度器支持三种模式：**interval**（固定间隔）、**cron**（cron 表达式）、**at**（具体时刻列表）。三者在 daemon 子命令中互斥。任务实例持久化到 `config/schedule.toml`，重启后能识别"已执行"与"未执行"。

## 三种模式

### Interval（固定间隔）

```
--interval 30m    # 每 30 分钟一次
--interval 2h     # 每 2 小时一次
```

**实现**：用 `tokio::time::interval(Duration)`；首次立即触发，之后按固定周期。

### Cron（cron 表达式）

```
--cron "*/15 * * * *"    # 每 15 分钟
--cron "0 9 * * *"        # 每天 9 点
--cron "0 0 * * 1"        # 每周一 0 点
```

**支持**：分 时 日 月 周（5 字段，标准 cron）。
**不支持**：`?`、`L`、`W`、`#`（Quartz 扩展）。

**解析**：自实现轻量解析器（< 100 行），避免 `cron` crate 的重依赖。

### At（时刻列表）

```
--at 2026-09-01T09:00:00+08:00
--at 2026-09-02T09:00:00+08:00
--at 2026-09-03T09:00:00+08:00
```

**实现**：启动时按时间排序；到点触发；从列表移除；列表空则退出 daemon。

## Fixed prompt 中的占位符

`--mode fixed --prompt "..."` 的 prompt 在提交 ComfyUI 之前，会先做一次占位符扩展：

| 占位符 | 含义 |
|---|---|
| `${<dimension>}` | 从 `tags/<lang>/<dimension>.txt` 随机抽一个 tag 替换（按 seed 确定性） |

**示例**：

```bash
auto-comfy-maker daemon \
  --at "$(date -u +%Y-%m-%dT%H:%M:%S+00:00)" \
  --mode fixed \
  --prompt '1girl, ${art_style}, ${background}, masterpiece' \
  --lang en \
  --template anima-lora
```

每执行一次，`${art_style}` 和 `${background}` 都会从对应 tag 池抽一个填进去，其余文本原样保留。

**未匹配到的维度**原样保留（不静默吞掉，便于调试）。

## 调度器架构

```
┌──────────────────────────────────────────┐
│           Daemon Loop (tokio)            │
├──────────────────────────────────────────┤
│ 1. 加载 schedule.toml（已执行记录）       │
│ 2. 根据模式启动对应 trigger               │
│ 3. trigger 触发 → 调度 task              │
│ 4. task 执行：                            │
│    ├─ 调用 prompt_engine                  │
│    ├─ 调用 comfyui client                 │
│    └─ 写 schedule.toml（标记已执行）      │
│ 5. 异常处理 + 重试（指数退避）            │
└──────────────────────────────────────────┘
```

## 持久化格式

```toml
# config/schedule.toml（自动生成，gitignore）

# mode = fixed（固定 prompt）
[[jobs]]
id = "550e8400-e29b-41d4-a716-446655440000"
scheduled_at = "2026-09-01T09:00:00+08:00"
status = "completed"
mode = "fixed"
prompt = "长发美女在海边"          # 直接保存 prompt 文本
completed_at = "2026-09-01T09:00:01+08:00"
output_path = "output/2026-09-01/img_xxx.png"

# mode = auto（自动随机组合）
[[jobs]]
id = "..."
scheduled_at = "2026-09-01T09:30:00+08:00"
status = "pending"
mode = "auto"
# auto 模式不保存 prompt——每次从 tags 重新随机抽取
```

**字段说明**：
- `status = "pending"`：待执行（启动时若过期则立即执行并标 completed）。
- `status = "running"`：执行中（防止并发）。
- `status = "completed"`：已完成。
- `status = "failed"`：失败（保留记录，便于排查）。
- `mode = "fixed" | "auto"`：与 daemon `--mode` 一致；`fixed` 时必填 `prompt` 字段，`auto` 时无 `prompt` 字段。

## 重试策略

```
失败 →
  ├─ 第 1 次：立即重试
  ├─ 第 2 次：30s 后重试
  ├─ 第 3 次：2min 后重试
  └─ 第 4 次：标记 failed，停止重试
```

## 关键决策

### 决策 1：模式互斥而非组合

**为什么**：`--interval 30m --cron "0 9 * * *"` 语义模糊（是 AND 还是 OR？）。强制三选一让用户决策更明确。

### 决策 2：自实现 cron 解析

**为什么**：`cron` crate 体积大、支持 Quartz 扩展（本项目不需要）。自实现 < 100 行，覆盖 5 字段即可。

### 决策 3：持久化用 toml 而非 sqlite

**为什么**：任务量小（通常 < 100 条/天），toml 读写肉眼可调试；sqlite 引入 `rusqlite` 依赖与异步封装成本。

### 决策 4：失败重试带指数退避

**为什么**：ComfyUI 偶发超时是常见现象，重试能显著提升成功率；纯固定间隔重试可能在服务恢复前浪费请求。

## 反模式

- ❌ 在调度器里直接构造 HTTP 请求（应通过 comfyui client）。
- ❌ 用线程 sleep 控制节奏（应使用 tokio::time）。
- ❌ 任务执行失败后 silently 继续（应记录到 schedule.toml）。
- ❌ daemon 启动时不区分"过期 pending"与"未来 pending"，导致全部补跑。