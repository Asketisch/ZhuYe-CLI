# Configuration Reference

<p align="center">  <a href="configuration.md">中文</a> · <b>English</b>  </p>

> The TUI's CLI flags, data directory, environment variables, and the
> commonly used config.toml sections. The complete field-level definition of
> config.toml is provided by the core engine — see the submodule's
> [zhuye-agent/docs/configuration.en.md](../zhuye-agent/docs/configuration.en.md).
> For commands and key bindings see [slash-commands.md](slash-commands.en.md);
> for architecture see [architecture.md](architecture.en.md).

## 1. Data Directory `~/.zhuye/`

User data is concentrated in `~/.zhuye/` (the engine-side `ZHUYE_HOME` can
relocate the whole directory):

| Path | Contents |
|------|------|
| `config.toml` | Main configuration (provider / permissions / sandbox / telemetry, etc.); hot-reloaded on change, no restart needed |
| `sessions/` | Session rollouts (JSONL); `_names/<id>.name` holds human-readable session names |
| `traces/` | LLM call records `model-io-sess_<session_id>.jsonl` (written by zhuye-telemetry) |
| `logs/` | TUI runtime logs `tui-<date>.log` (attach the relevant snippet when reporting issues; remember to redact) |
| `permissions.toml` | Runtime always-allow permission persistence |

Workspace-local files (not part of user data):

| Path | Contents |
|------|------|
| `<workspace>/.zhuye/plan/<name>.md` | Plan mode drafts (written by PlanWrite, read by ExitPlanMode) |
| `<workspace>/.zhuye/prompts/*.md` | Session prompt layers (listed / switched via `/prompt`; `/prompt reload` rescans after edits) |

## 2. CLI Flags

Launching without a subcommand starts the TUI. Common flags (see
`zhuye-cli --help` for the full list):

```bash
zhuye-cli [--cwd PATH] [--prompt TEXT] [-c] [-r N] [--resume ID]
            [--plan-mode] [--fullscreen] [--auto-root]
            [--ephemeral-tasks] [--ephemeral-teams]

# Examples
zhuye-cli "refactor this module for me"   # launch with an initial prompt
zhuye-cli --resume sess_abc123            # resume a specific session
zhuye-cli -c                              # continue the last session
zhuye-cli -r 2                            # resume the 2nd most recent session
zhuye-cli --plan-mode                     # start in Plan mode
zhuye-cli --fullscreen                    # alternate-screen fullscreen (also ZHUYE_CLI_FULLSCREEN=1)
zhuye-cli --cwd /path/to/project          # set the working directory
```

With a subcommand (`session` / `traces`) the binary runs headlessly and exits
when done; see the "CLI subcommands" section of the README for usage.

## 3. Environment Variables

| Variable | Purpose |
|------|------|
| `OPENAI_API_KEY` / `ANTHROPIC_API_KEY` | Env fallback credentials for the same-named model groups (`id = "openai"` / `"anthropic"`); pick one to get started (since v2 the fixed access sections are gone; other groups never read env) |
| `ZHUYE_MODEL` | Override the model spec; `=mock/mock-1` selects the built-in mock provider — offline, zero network |
| `ZHUYE_HOME` | Data directory, defaults to `~/.zhuye` |
| `ZHUYE_CLI_FULLSCREEN` | `=1` enables alternate-screen fullscreen mode (equivalent to `--fullscreen`) |
| `ZHUYE_SANDBOX_OS_LEVEL` | `=1` enables the OS-level sandbox (macOS Seatbelt / Linux Landlock) |
| `RUST_LOG` | Log filtering, defaults to `warn,zhuye=info` |

## 4. Common config.toml Sections

Since v2 (upstream 8da90e5) the config surface is **model-group only**:
`[[model_group]]` is the single unit of model access. The v1 fixed sections
(`[anthropic]` / `[openai]` / `[ollama]`) no longer appear in written-back
configs — old configs are migrated automatically into equivalent model groups
on load, no manual edit required.

```toml
[active]
group = "openai"                    # active model-group id (falls back by
                                    # resolution order when absent:
                                    # env ZHUYE_PROVIDER → ZHUYE_MODEL mock
                                    # special-case → [active].group →
                                    # ZHUYE_MODEL prefix → first credentialed group)
# bundled_skills = true            # explicitly enable the bundled skill pack (off by default)

[[model_group]]                     # group id = model spec prefix (<id>/<model>)
id = "openai"                       # required, unique, no '/'; group id = registry pool name
endpoint = "openai"                 # wire format: openai | anthropic | ollama
api_key = ""                        # can stay empty for keyless endpoints
base_url = "https://api.kilo.ai/api/gateway"
model = "free"                      # "free" resolves to a real model at runtime
temperature = 0.3
max_tokens = 4096
timeout_secs = 300                  # HTTP timeout: connect + read-idle, two layers

[model]
context_window = 128000             # context ceiling (tokens); compaction thresholds follow
reasoning_effort = "high"           # thinking effort, clamped per model family before the wire

[routing]
max_attempts = 10                   # total attempt ceiling per model call (incl. auto retries)
```

Other commonly used sections (field-level definitions in the engine-side
document):

| Section | Purpose |
|------|------|
| `[permissions]` | Read-only permission rules (resolved as a chain with `permissions.toml`; MCP / plugin-sourced tools are forced to the `Prompt` floor) |
| `[sandbox]` | `os_level` toggles the OS-level sandbox |
| `[sanitize]` | Persistence redaction (10 secret-pattern classes, extensible) |
| `[notifications.tui]` | Terminal notification toggles and channels |
| `[telemetry]` | `redact` / `max_bytes` / `record_tool_schemas`, etc. |
| `[model_discovery]` | Runtime resolution rules for `model = "free"` |
| `[plugins]` | `enabled_plugins` / `disabled_plugins` — gated bundled plugin toggles (web-search / notebook on by default; explicit disable = tool unregistered) |
| `[subagent_models]` | Subagent-only model endpoints (v1 section name `[subagent_providers]`; the old name migrates on load) |

## 5. Further Reading

- [zhuye-agent/docs/configuration.en.md](../zhuye-agent/docs/configuration.en.md) —— complete config.toml field definitions (external link, submodule; includes the v1 → v2 migration table)
- [slash-commands.md](slash-commands.en.md) —— runtime adjustment entry points such as `/model` and `/permissions`
- [SECURITY.md](../SECURITY.en.md) —— the security perspective on the permission chain, sandbox, and redaction
