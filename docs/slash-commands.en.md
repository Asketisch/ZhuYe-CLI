# Slash Commands and Key Bindings Reference

<p align="center">  [中文](slash-commands.md) · **English**  </p>

> The runtime-outcome reference for every slash command inside the TUI, plus
> common key bindings. The outcome categories mirror the `MATRIX` in
> `tests/functional/test_slash.py` — that matrix is the single source of
> truth, and this document is its user-facing projection; both are updated
> together when commands are added or changed. For architectural background
> see [architecture.md](architecture.en.md); for configuration see
> [configuration.md](configuration.en.md).

## 1. Outcome Categories

Typing `/` opens the command list. Every command has a well-defined runtime
outcome in the current build:

| Category | Meaning |
|---|---|
| `wired` | Wired up with real behavior (switch models, enter goal mode, compact the context, etc.) |
| `panel` | Opens a panel / overlay (diff, tasks, transcript replay, pickers, etc.) |
| `placeholder` | The command is recognized but not wired into dispatch in this build; a placeholder notice is shown |
| `unrecognized` | The command is not recognized and the submission is silently swallowed (see the note below) |
| `exit` | Exits the TUI |

> `unrecognized` is a known behavior of the current implementation: the
> composer only admits "commands visible in the enum" plus legacy-whitelisted
> ones; other submissions are dropped and the user just sees the input still
> sitting in the box.

## 2. Wired Commands

| Command | Behavior |
|------|------|
| `/model [spec]` | Switch model and reasoning strength; without arguments, lists the provider's real model catalog |
| `/vim` | Enable Vim input mode |
| `/status` | Current session configuration and token usage (per model) |
| `/usage` | Usage statistics |
| `/cost` | Cost statistics |
| `/effort <level>` | Set reasoning effort (low/medium/high/…) |
| `/compact` | Request context compaction to avoid hitting the context limit |
| `/goal <text>` | Enter goal mode; without arguments, shows a goal-state snapshot |
| `/loop <secs> <cmd>` | Repeat a command at an interval (local scheduler; `/loop stop` cancels) |
| `/side <message>` | Side conversation (does not enter the main flow) |
| `/btw <message>` | Alias for the side conversation |
| `/rewind` | Drop the last turn (rollout gets a `.bak` backup first) |
| `/clear` | Clear the screen and confirm starting a new chat |
| `/rm-session` | Permanently delete the current session files (with confirmation) |
| `/plan <goal>` | Initiate a Plan mode request (draft plan first, then approve and execute) |
| `/mode <mode>` | Switch working mode (e.g. `plan`) |
| `/permissions` | Cycle permission modes (Auto → Ask → …) |
| `/exit-plan` / `/exitplan` | Leave Plan mode (choose how to execute) |
| `/ref` | Switch reference search modes |
| `/mcp` | Show MCP server status (with curated-catalog overlay) |
| `/lsp` | Language-server diagnostics status (document count / diagnostic count / server names) |
| `/todo` | Show the agent's current todo list |
| `/subagents` | Show sub-agents |
| `/skills` | Skills management (this build reports "not wired") |
| `/hooks` | Hooks browsing (this build reports "not wired") |

## 3. Panel / Overlay Commands

| Command | Panel opened |
|------|------|
| `/diff` | git diff panel (including untracked files; Esc / Ctrl+T to close) |
| `/plugin` / `/plugins` | Installed-plugins panel |
| `/help` / `?` | Transcript replay (scroll through the full conversation) |
| `/tasks` / `/agents` | Tasks & agents panel |
| `/split` | Sub-agent split-view snapshot |
| `/prompt` | Session prompt-layer picker (lists available layers / the active layer) |

## 4. Placeholder Commands

The following commands are recognized but not wired into dispatch in the
current build; submitting them shows a "not wired into dispatch in this
build" placeholder notice:

| Command | Intended purpose |
|------|------|
| `/ide` | IDE integration |
| `/keymap` | Keymap configuration |
| `/experimental` | Experimental feature toggles |
| `/approve` | Approval settings |
| `/memories` | Memory management |
| `/import` | Import setup and history from external agents |
| `/review` | Review current changes |
| `/new` | Start a new chat during a conversation |
| `/archive` | Archive the current session and exit |
| `/delete` | Delete the current session and exit |
| `/resume` | Resume a saved chat |
| `/fork` | Fork the current chat |
| `/app` | Continue this session in the Desktop app |
| `/init` | Create an AGENTS.md file with project instructions |
| `/agent` | Sub-agent management |
| `/copy` | Copy the last response as markdown |
| `/raw` | Toggle raw scrollback mode for copy-friendly terminal selection |
| `/mention` | Mention a file |
| `/rename <name>` | Rename the current session |
| `/debug-config` | Show config layers and sources (debugging) |
| `/title` | Configure terminal-title items |
| `/statusline` | Configure status-line items |
| `/theme` | Choose a syntax-highlighting theme |
| `/pets` / `/pet` | Terminal pet |
| `/logout` | Log out |
| `/feedback` | Send logs to maintainers |
| `/ps` | Session / process list |
| `/stop` / `/clean` | Stop |
| `/personality` | Personality settings |
| `/debug-m-drop` / `/debug-m-update` | Memory debugging |

## 5. Unrecognized Commands

The following commands are **not recognized** in the current build; the
submission is silently swallowed (the input stays in the box, no notice):
`/setup-default-sandbox`, `/sandbox-add-read-dir`, `/apps`, `/rollout`,
`/test-approval`, `/version`, `/doctor`, `/history`, `/files`, `/branch`,
`/image` (`/img`), `/checkpoint`, `/traces`, `/think` (`/thinking`),
`/retry`.

## 6. Exit Commands

| Command | Behavior |
|------|------|
| `/exit` / `/quit` | Exit the TUI (process exit code 0) |

## 7. Common Key Bindings

The following key behaviors are locked by tests
(`tests/functional/test_keys.py`):

| Key | Behavior |
|------|------|
| `Enter` | Submit the current input |
| `Alt+Enter` | Newline (multi-line input) |
| `Shift+Tab` | Cycle permission modes |
| `Ctrl+O` | Copy the last reply |
| `Ctrl+G` | Edit the input in an external editor (fullscreen mode leaves / re-enters the alternate screen automatically) |
| `Ctrl+R` | Search input history |
| `↑` | Recall the previous input-history entry |
| `Ctrl+A` | Move the cursor to the start of the line |
| `Esc` | Close overlays / panels / pickers |
| `Ctrl+T` | Close panels (diff / transcript / tasks, etc.) |
| Large pastes | Chunked pasting stays coherent (bracketed-paste) |

With Vim mode enabled (`/vim`), Vim editing keys work in normal state.
