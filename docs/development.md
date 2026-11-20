# 开发指南

<p align="center">  <b>中文</b> · <a href="development.en.md">English</a>  </p>

> 构建、测试分层与验证流程。贡献前的完整规范(提交约定、submodule 工作流、
> PR 要求)见 [CONTRIBUTING.md](../CONTRIBUTING.md);架构背景见
> [architecture.md](architecture.md);发版流程见 [release.md](release.md)。

## 1. 开发环境

```bash
git clone --recursive https://github.com/Asketisch/Reflect-TUI.git
cd ZhuYe-CLI
make build                      # cargo build --release -p zhuye-cli
```

- Rust stable,edition 2024,MSRV 1.85(`rust-toolchain.toml` 锁定);
- 离线开发:`ZHUYE_MODEL=mock/mock-1` + 隔离 HOME(mktemp),不污染真实
  `~/.zhuye/`;
- 能力覆盖层需要 Python 3 + `pytest` + `pyte`(`pip install pytest pyte`)。

## 2. 常用命令

```bash
make build                      # release 构建
make run                        # 构建并启动 TUI
make check / make check-fast C=zhuye-cli   # 类型检查(全 workspace / 单 crate)
make test                       # cargo test --workspace
make test-fast                  # cargo nextest(需已安装 cargo-nextest)
make test-suite                 # ./tests/run_all.sh:全功能套件
make test-suite-fast            # 只跑能力覆盖层(跳过 cargo 编译面)
make coverage-audit             # 覆盖自审:slash 命令 × 测试矩阵差集核对
make gc                         # 清产物根下全部 profile 的 incremental 缓存(不触发重编)
make gc-deep                    # 深清理:再清 deps/.fingerprint/examples(下次构建接近冷构建)
```

任何改动提交前:`cargo fmt --all -- --check` +
`cargo clippy --workspace --all-targets -- -D warnings`。

## 3. 测试分层

`tests/` 目录集中存放全部测试脚本,分三层:

| 层 | 落点 | 语义 |
|---|---|---|
| 能力覆盖层 | `tests/functional/`(Python) | 真二进制 + pty 驱动,断言到**行为级**(屏幕可见文本 / 落盘文件内容 / 退出码),按面分组(B 启动 / S slash / O 弹层 / T 轮次 / P 会话 / C CLI / K 按键 / V 渲染 / W 流程 / G 插件) |
| Rust 参数面 | `tests/rust/*.sh` | CLI 子命令的**参数面**(只管参数解析,不重复行为断言) |
| 覆盖自审 | `tests/tools/coverage_audit.py` | 静态核对「Rust 里声明的 slash 命令」×「测试矩阵覆盖」,缺口即失败 |

入口与分层细则见 [tests/README.md](../tests/README.md),能力 × 覆盖层
映射见 [tests/COVERAGE.md](../tests/COVERAGE.md)(均为内部文档,中文)。

**新功能或行为修改必须与测试脚本一起提交**:改了 TUI 对外可见的行为,
必须在 `tests/functional/` 有对应用例;新增 slash 命令要同步
`tests/functional/test_slash.py` 的 `MATRIX`,否则 `make coverage-audit`
直接失败。

## 4. 验证矩阵(按改动范围选跑)

| 改动范围 | 必跑命令 |
|---|---|
| 任何改动 | `cargo fmt --all -- --check` + `cargo clippy --workspace --all-targets -- -D warnings` |
| Rust 代码 / crate | `cargo test --workspace` |
| TUI 行为(渲染 / 按键 / 弹层 / 轮次 / 落盘 / CLI 语义) | `./tests/run_all.sh functional` |
| slash 命令 | `./tests/run_all.sh functional` + `make coverage-audit` |
| CLI 子命令参数面 | `./tests/run_all.sh rust` |
| 仅文档 / 注释 | 无需跑测试,自查链接与命令是否仍有效 |
| 发版前 | `make release-check`(见 [release.md](release.md)) |

## 5. 离线与 live 测试

- 离线全量:`./tests/run_all.sh`(能力覆盖层 + Rust 编译面 + 覆盖自审),
  全程 `ZHUYE_MODEL=mock/mock-1` + 隔离 HOME;
- live 端到端:`./tests/run_all.sh live`(真实 LLM + 真实工具,需 API key,
  手动跑),回归 Plan mode / 子代理 / `/goal` / `/loop` 完整链路。
