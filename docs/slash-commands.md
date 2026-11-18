# Slash 命令与按键参考

<p align="center">  <b>中文</b> · [English](slash-commands.en.md)  </p>

> TUI 内全部 slash 命令的运行时结局参考,以及常用按键。结局口径与
> `tests/functional/test_slash.py` 的 `MATRIX` 一致 —— 该矩阵是单一事实源,
> 本文是它的用户视角投影,新增 / 改动命令时两边同步。架构背景见
> [architecture.md](architecture.md);配置见 [configuration.md](configuration.md)。

## 1. 命令结局分类

输入 `/` 弹出命令列表。每条命令在当前构建中都有明确的运行时结局:

| 类别 | 含义 |
|---|---|
| `wired` | 已接线,有真实行为(切换模型、开目标模式、压缩上下文等) |
| `panel` | 打开面板 / 浮层(diff、任务、transcript 回放、选择器等) |
| `placeholder` | 命令被识别,但当前构建未接入分发,显示占位提示 |
| `unrecognized` | 命令未被识别,提交被静默吞掉(输入留在框里,见下方说明) |
| `exit` | 退出 TUI |

> `unrecognized` 是当前实现的已知行为:composer 只放行「枚举里可见的命令」
> 与 legacy 白名单命令,其余命令的提交被丢弃,用户只看到输入还在框里。

## 2. 已接线命令(wired)

| 命令 | 行为 |
|------|------|
| `/model [spec]` | 切换模型与推理强度;无参列出 provider 真实模型清单 |
| `/vim` | 启用 Vim 输入模式 |
| `/status` | 当前会话配置与 token 用量(按模型统计) |
| `/usage` | 用量统计 |
| `/cost` | 费用统计 |
| `/effort <level>` | 设置推理强度(low/medium/high/…) |
| `/compact` | 请求压缩上下文,防止撞上下文上限 |
| `/goal <text>` | 进入目标模式;无参显示目标状态快照 |
| `/loop <secs> <cmd>` | 按间隔重复执行某条命令(本地调度,`/loop stop` 取消) |
| `/side <message>` | 侧边对话(不进主对话流) |
| `/btw <message>` | 侧边对话的别名 |
| `/rewind` | 回退最后一个 turn(rollout 先写 `.bak` 备份) |
| `/clear` | 清屏并确认开新对话 |
| `/rm-session` | 永久删除当前会话文件(带确认) |
| `/plan <goal>` | 发起 Plan mode 请求(先出 plan 草稿,审批后执行) |
| `/mode <mode>` | 切换工作模式(如 `plan`) |
| `/permissions` | 轮换权限模式(Auto → Ask → …) |
| `/exit-plan` / `/exitplan` | 退出 Plan mode(选择执行方式) |
| `/ref` | 切换引用搜索模式 |
| `/mcp` | 查看 MCP server 状态(含精选目录 overlay) |
| `/lsp` | 语言服务诊断状态(文档数 / 诊断条数 / server 名) |
| `/todo` | 查看 agent 当前待办清单 |
| `/subagents` | 查看子代理 |
| `/skills` | 技能管理(当前构建提示未接入) |
| `/hooks` | hooks 浏览(当前构建提示未接入) |

## 3. 面板 / 浮层命令(panel)

| 命令 | 打开的面板 |
|------|------|
| `/diff` | git diff 面板(含未跟踪文件;Esc / Ctrl+T 关闭) |
| `/plugin` / `/plugins` | 已安装插件面板 |
| `/help` / `?` | transcript 回放(滚动查看完整对话) |
| `/tasks` / `/agents` | 任务与子代理面板 |
| `/split` | 子代理分屏快照视图 |
| `/prompt` | 会话提示词层选择(列出可用层 / 当前生效层) |

## 4. 占位命令(placeholder)

以下命令被识别,但当前构建未接入分发,提交后显示「尚未接入分发」占位提示:

| 命令 | 预期用途 |
|------|------|
| `/ide` | IDE 集成 |
| `/keymap` | 按键映射配置 |
| `/experimental` | 实验特性开关 |
| `/approve` | 审批设置 |
| `/memories` | 记忆管理 |
| `/import` | 从外部 agent 导入配置与历史 |
| `/review` | 审查当前变更 |
| `/new` | 会话中开新对话 |
| `/archive` | 归档当前会话并退出 |
| `/delete` | 删除当前会话并退出 |
| `/resume` | 恢复已保存会话 |
| `/fork` | fork 当前会话 |
| `/app` | 在 Desktop 应用中继续本会话 |
| `/init` | 生成 AGENTS.md 项目说明文件 |
| `/agent` | 子代理管理 |
| `/copy` | 复制最后一条回复为 markdown |
| `/raw` | 切换原始滚动模式(便于终端选区复制) |
| `/mention` | 提及文件 |
| `/rename <name>` | 重命名当前会话 |
| `/debug-config` | 显示配置层与来源(调试) |
| `/title` | 配置终端标题项 |
| `/statusline` | 配置状态栏项 |
| `/theme` | 选择语法高亮主题 |
| `/pets` / `/pet` | 终端宠物 |
| `/logout` | 登出 |
| `/feedback` | 发送日志给维护者 |
| `/ps` | 会话 / 进程列表 |
| `/stop` / `/clean` | 停止 |
| `/personality` | 人格设置 |
| `/debug-m-drop` / `/debug-m-update` | 记忆调试 |

## 5. 未识别命令(unrecognized)

以下命令在当前构建中**未被识别**,提交会被静默吞掉(输入留在框里、无提示):
`/setup-default-sandbox`、`/sandbox-add-read-dir`、`/apps`、`/rollout`、
`/test-approval`、`/version`、`/doctor`、`/history`、`/files`、`/branch`、
`/image`(`/img`)、`/checkpoint`、`/traces`、`/think`(`/thinking`)、`/retry`。

## 6. 退出命令

| 命令 | 行为 |
|------|------|
| `/exit` / `/quit` | 退出 TUI(进程退出码 0) |

## 7. 常用按键

以下按键行为由测试锁定(`tests/functional/test_keys.py`):

| 按键 | 行为 |
|------|------|
| `Enter` | 提交当前输入 |
| `Alt+Enter` | 换行(多行输入) |
| `Shift+Tab` | 轮换权限模式 |
| `Ctrl+O` | 复制最后一条回复 |
| `Ctrl+G` | 外部编辑器编辑输入(全屏模式自动离开 / 重进备用屏) |
| `Ctrl+R` | 搜索输入历史 |
| `↑` | 翻回上一条输入历史 |
| `Ctrl+A` | 光标到行首 |
| `Esc` | 关闭弹层 / 面板 / 选择器 |
| `Ctrl+T` | 关闭面板(diff / transcript / tasks 等) |
| 大段粘贴 | 分块粘贴保持连贯(bracketed-paste) |

Vim 模式(`/vim` 启用)下支持 normal 状态的 Vim 编辑键。
