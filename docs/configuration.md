# 配置参考

<p align="center">  <b>中文</b> · <a href="configuration.en.md">English</a>  </p>

> TUI 的启动参数、数据目录、环境变量与 config.toml 常用节。config.toml 的
> 完整字段定义由核心引擎提供,见 submodule 的
> [zhuye-agent/docs/configuration.md](../zhuye-agent/docs/configuration.md)。
> 命令与按键参考见 [slash-commands.md](slash-commands.md);架构见
> [architecture.md](architecture.md)。

## 1. 数据目录 `~/.zhuye/`

用户数据集中在 `~/.zhuye/`(引擎侧 `ZHUYE_HOME` 可整体挪走):

| 路径 | 内容 |
|------|------|
| `config.toml` | 主配置(provider / 权限 / 沙箱 / 遥测等),改完热重载,无需重启 |
| `sessions/` | 会话 rollout(JSONL);`_names/<id>.name` 存人可读会话名 |
| `traces/` | LLM 调用记录 `model-io-sess_<session_id>.jsonl`(zhuye-telemetry 写入) |
| `logs/` | TUI 运行日志 `tui-<日期>.log`(报告问题时附相关片段,注意脱敏) |
| `permissions.toml` | 运行期 always-allow 权限落盘 |

工作区本地文件(不入用户数据目录):

| 路径 | 内容 |
|------|------|
| `<workspace>/.zhuye/plan/<name>.md` | Plan mode 草稿(PlanWrite 写、ExitPlanMode 读) |
| `<workspace>/.zhuye/prompts/*.md` | 会话提示词层(`/prompt` 列出 / 切换,改完 `/prompt reload` 重扫) |

## 2. 启动参数

无子命令默认启动 TUI。常用参数(完整列表见 `zhuye-cli --help`):

```bash
zhuye-cli [--cwd PATH] [--prompt TEXT] [-c] [-r N] [--resume ID]
            [--plan-mode] [--fullscreen] [--auto-root]
            [--ephemeral-tasks] [--ephemeral-teams]

# 示例
zhuye-cli "帮我重构这个模块"        # 直接带 prompt 启动
zhuye-cli --resume sess_abc123     # 恢复指定 session
zhuye-cli -c                       # 继续上一次会话
zhuye-cli -r 2                     # 恢复最近第 2 个 session
zhuye-cli --plan-mode              # 以 Plan mode 启动
zhuye-cli --fullscreen             # 全屏模式(备用屏;也可用 ZHUYE_CLI_FULLSCREEN=1)
zhuye-cli --cwd /path/to/project   # 指定工作目录
```

带子命令(`session` / `traces`)走 headless 路径,执行完即退出,用法见
README 的「CLI 子命令」一节。

## 3. 环境变量

| 变量 | 作用 |
|------|------|
| `OPENAI_API_KEY` / `ANTHROPIC_API_KEY` | 同名模型组(`id = "openai"` / `"anthropic"`)的 env 兜底凭据,起步二选一(v2 起固定接入段移除,其余组不读 env) |
| `ZHUYE_MODEL` | 覆盖模型规格;`=mock/mock-1` 为内置 mock provider,离线零网络 |
| `ZHUYE_HOME` | 数据目录,默认 `~/.zhuye` |
| `ZHUYE_CLI_FULLSCREEN` | `=1` 时启用全屏备用屏模式(等价 `--fullscreen`) |
| `ZHUYE_SANDBOX_OS_LEVEL` | `=1` 时启用 OS 级沙箱(macOS Seatbelt / Linux Landlock) |
| `RUST_LOG` | 日志过滤,默认 `warn,zhuye=info` |

## 4. config.toml 常用节

v2(上游 8da90e5)起配置面是**模型组唯一**:`[[model_group]]` 是唯一的模型
接入单元,v1 的固定段(`[anthropic]` / `[openai]` / `[ollama]`)不再出现在
写回格式里 —— 旧配置读入后由引擎自动迁移成等价模型组,无需手动改。

```toml
[active]
group = "openai"                    # 活跃模型组 id(缺省按解析顺序兜底:
                                    # env ZHUYE_PROVIDER → ZHUYE_MODEL mock 特判
                                    # → [active].group → ZHUYE_MODEL 前缀 →
                                    # 首个有凭据的组)
# bundled_skills = true            # 显式开启内置 skill 包(默认关闭)

[[model_group]]                     # 组 id = 模型 spec 前缀(<id>/<model>)
id = "openai"                       # 必填、唯一、不含 '/';组 id = registry 池名
endpoint = "openai"                 # 线格式:openai | anthropic | ollama
api_key = ""                        # 免 key 端点可以留空
base_url = "https://api.kilo.ai/api/gateway"
model = "free"                      # "free" 运行时从清单解析出真实模型
temperature = 0.3
max_tokens = 4096
timeout_secs = 300                  # HTTP 超时:建连 + 读空闲双层

[model]
context_window = 128000             # 上下文上限(token),压缩阈值跟着走
reasoning_effort = "high"           # 思考强度,wire 前按模型族钳制

[routing]
max_attempts = 10                   # 单次模型调用总尝试上限(含自动重试)
```

其余常用节(字段级定义见引擎侧文档):

| 节 | 作用 |
|------|------|
| `[permissions]` | 只读权限规则(与 `permissions.toml` 链式解析;MCP / plugin 来源工具强制 `Prompt` 下限) |
| `[sandbox]` | `os_level` 开关 OS 级沙箱 |
| `[sanitize]` | 落盘脱敏(10 类密钥 pattern 可扩展) |
| `[notifications.tui]` | 终端通知开关与通道 |
| `[telemetry]` | `redact` / `max_bytes` / `record_tool_schemas` 等 |
| `[model_discovery]` | `model = "free"` 的运行时解析规则 |
| `[plugins]` | `enabled_plugins` / `disabled_plugins` —— 门控型 bundled 插件开关(web-search / notebook 默认开,显式禁用 = 工具不进注册表) |
| `[subagent_models]` | 子代理专用模型端点(v1 段名 `[subagent_providers]`,旧名读入时自动迁移) |

## 5. 深入阅读

- [zhuye-agent/docs/configuration.md](../zhuye-agent/docs/configuration.md) —— config.toml 完整字段定义(外链,submodule;含 v1 → v2 迁移表)
- [slash-commands.md](slash-commands.md) —— `/model`、`/permissions` 等运行时调整入口
- [SECURITY.md](../SECURITY.md) —— 权限链、沙箱与脱敏的安全视角
