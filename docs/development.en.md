# Development Guide

<p align="center">  <a href="development.md">中文</a> · <b>English</b>  </p>

> Builds, test layering, and the verification workflow. The full
> contribution rules (commit conventions, submodule workflow, PR
> requirements) are in [CONTRIBUTING.md](../CONTRIBUTING.md); architectural
> background is in [architecture.md](architecture.en.md); the release
> process is in [release.md](release.en.md).

## 1. Development Environment

```bash
git clone --recursive https://github.com/Asketisch/Reflect-TUI.git
cd ZhuYe-CLI
make build                      # cargo build --release -p zhuye-cli
```

- Rust stable, edition 2024, MSRV 1.85 (pinned by `rust-toolchain.toml`);
- Offline development: `ZHUYE_MODEL=mock/mock-1` + an isolated HOME (mktemp),
  leaving the real `~/.zhuye/` untouched;
- The capability coverage layer needs Python 3 + `pytest` + `pyte`
  (`pip install pytest pyte`).

## 2. Common Commands

```bash
make build                      # release build
make run                        # build and launch the TUI
make check / make check-fast C=zhuye-cli   # type-check (whole workspace / single crate)
make test                       # cargo test --workspace
make test-fast                  # cargo nextest (requires cargo-nextest)
make test-suite                 # ./tests/run_all.sh: full functional suite
make test-suite-fast            # capability coverage only (skips the cargo compile surface)
make coverage-audit             # coverage audit: slash commands × test-matrix diff
make gc                         # clear incremental caches under all profiles in the target dir (no rebuild)
make gc-deep                    # deep clean: also drop deps/.fingerprint/examples (next build ~ cold build)
```

Before submitting any change: `cargo fmt --all -- --check` +
`cargo clippy --workspace --all-targets -- -D warnings`.

## 3. Test Layering

The `tests/` directory holds all test scripts in three layers:

| Layer | Location | Semantics |
|---|---|---|
| Capability coverage | `tests/functional/` (Python) | Drives the **real binary over a pty** with **behavior-level** assertions (visible screen text / persisted file contents / exit codes), grouped by surface (B boot / S slash / O overlays / T turns / P sessions / C CLI / K keys / V rendering / W flows / G plugins) |
| Rust argument surface | `tests/rust/*.sh` | The CLI subcommand **argument surface** (argument parsing only; no duplicated behavior assertions) |
| Coverage audit | `tools/coverage_audit.py` | Statically diffs "slash commands declared in Rust" against "commands covered by the matrix"; any gap fails |

Entry points and layering details are in [tests/README.md](../tests/README.md)
and the capability × coverage mapping in [tests/COVERAGE.md](../tests/COVERAGE.md)
(internal documents, in Chinese).

**New features or behavior changes must be submitted together with their test
scripts**: any change to externally visible TUI behavior needs a
corresponding case in `tests/functional/`; a new slash command needs a row
in the `MATRIX` of `tests/functional/test_slash.py`, otherwise
`make coverage-audit` fails outright.

## 4. Verification Matrix (pick by change scope)

| Change scope | Required commands |
|---|---|
| Any change | `cargo fmt --all -- --check` + `cargo clippy --workspace --all-targets -- -D warnings` |
| Rust code / crate | `cargo test --workspace` |
| TUI behavior (rendering / keys / overlays / turns / persistence / CLI semantics) | `./tests/run_all.sh functional` |
| Slash commands | `./tests/run_all.sh functional` + `make coverage-audit` |
| CLI subcommand argument surface | `./tests/run_all.sh rust` |
| Docs / comments only | No tests needed; self-check that links and commands still work |
| Before a release | `make release-check` (see [release.md](release.en.md)) |

## 5. Offline and Live Testing

- Full offline run: `./tests/run_all.sh` (capability coverage + Rust compile
  surface + coverage audit), entirely with `ZHUYE_MODEL=mock/mock-1` + an
  isolated HOME;
- Live end-to-end: `./tests/run_all.sh live` (real LLM + real tools; needs an
  API key, run manually) to regress the full Plan mode / sub-agents / `/goal`
  / `/loop` chains.
