# 配置文件目录

本目录存放运行时配置。详见 `docs/config.md`。

## 文件清单

| 文件 | 提交 | 说明 |
|------|------|------|
| `default.toml` | ✅ | 仓库默认配置，所有人都用 |
| `local.toml` | ❌ | 本地覆盖，gitignore |
| `schedule.toml` | ❌ | 定时任务持久化，gitignore |

## 优先级

```
环境变量 > CLI 参数 > local.toml > default.toml
```

## 初始化

仓库首次克隆后，只需确认 `default.toml` 存在即可。`local.toml` 由用户按需创建。