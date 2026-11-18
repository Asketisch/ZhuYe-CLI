# Security Policy

<p align="center">  <a href="SECURITY.md">中文</a> · <b>English</b>  </p>

> Vulnerability reporting and handling process for ZhuYe-CLI, plus
> security-relevant design notes for auditors. The sandbox / permissions /
> redaction mechanisms are provided by the core engine (submodule); this
> document only covers the perspective and boundaries of this TUI repository.

## Supported Versions

| Version | Supported |
| ---- | ---- |
| latest release | ✅ |
| older versions | ❌ (please upgrade first) |

## Reporting a Vulnerability

**Do not report security vulnerabilities through public issues.**

Please use GitHub's [private vulnerability reporting](https://github.com/Asketisch/Reflect-TUI/security/advisories/new)
(Security → Advisories → New draft security advisory).

When reporting, please include as much as possible:

- Affected version / commit
- Reproduction steps or PoC
- Impact assessment (data leakage / arbitrary execution / privilege escalation, etc.)
- Suggested mitigations (if any)

We will acknowledge receipt within **72 hours** and coordinate a disclosure
timeline with you before the fix lands.

## Security-Relevant Design Notes (for auditors)

- **Sandbox**: the bash tool supports macOS Seatbelt / Linux Landlock
  (`[sandbox].os_level = true` or `ZHUYE_SANDBOX_OS_LEVEL=1`).
- **Permission chain**: `~/.zhuye/config.toml [permissions]` (read-only,
  flattened) + `~/.zhuye/permissions.toml` (runtime always-allow persistence)
  → resolved as a chain; tools originating from MCP / plugin / runtime sources
  are forced to the `Prompt` safety floor, preventing static blanket approvals.
- **Redaction**: rollout persistence and tool output go through the `Sanitizer`
  by default (10 classes of secret patterns, extensible via `[sanitize]`),
  with a 16 KiB truncation.
- **Trust boundary**: the TUI is an in-process host; the externally exposed
  surface of the wire protocol (Submission / EventMsg) is the engine
  repository's `zhuye serve` stdio mode; MCP / LSP servers are external
  processes launched only through explicit config.toml entries.
- **Dependencies**: weekly `cargo audit` (audit.yml); the forked ratatui /
  crossterm git dependencies are outside crates.io scanning scope and need
  manual follow-up via GSA.

## Known Trade-offs

- The pty test harness (`test/`) and the mock provider are for local testing
  only and should not be exposed to untrusted input environments.
