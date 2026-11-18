# 发版流程

<p align="center">  <b>中文</b> · <a href="release.en.md">English</a>  </p>

> 从发版前审查到 tag、打包、安装的完整链路。开发与测试环境见
> [development.md](development.md);变更记录约定见下文第 2 节。

## 1. 发版前审查

发版前跑一次全量审查,覆盖**编译面 + 行为面 + 覆盖自审**:

```bash
make release-check
```

它依次执行:

1. `./tests/run_all.sh` —— Rust 编译面(`cargo test` / `clippy -D warnings` /
   `fmt` / `doc`)+ 能力覆盖层(真二进制 + pty 的行为断言 + CLI 参数矩阵);
2. `tests/tools/coverage_audit.py --strict` —— 确认「声明的 slash 命令」
   全部有行为级用例。

三者任一失败即**不应发版**。审查与打包是两步:先 `make release-check`,
再打包。

## 2. CHANGELOG 约定(受脚本机械解析,格式勿改)

`CHANGELOG.md` 采用 Keep a Changelog 框架,正文中文:

- 未发布变更写在 `## [Unreleased]` 小节下;
- 版本头格式固定为 `## [x.y.z] — YYYY-MM-DD`(em dash 分隔);
- 空小节占位符固定为 `(无)`;
- 发版时 `create_tag.sh` 会把 `[Unreleased]` 的内容**转正**为新版本小节
   —— 该脚本按上述标记**机械解析** `CHANGELOG.md`,改动标题语言、版本头
   格式或占位符会直接打断发版流程。

## 3. 打 tag(`./scripts/create_tag.sh`)

一站式发版:版本 bump + CHANGELOG 转正 + 提交 + annotated tag + 推送。

```bash
./scripts/create_tag.sh --dry-run    # 只打印不执行,验证流程首选
./scripts/create_tag.sh --no-push    # 本地打 tag 不推送
./scripts/create_tag.sh --yes        # 跳过交互确认
```

- 版本 bump 与 CHANGELOG 转正**必须同一次执行**完成;
- 推送 `v*` tag 触发 `.github/workflows/release.yml` 发布流水线;
- **测试环境不推真 tag**。

## 4. 打包与安装

`./scripts/build.sh`(跨平台构建):

- `--fast`:release-fast profile(无 LTO,增量编译快);
- `--dist`:构建 + 打包当前平台分发包(tar.gz / zip)到 `dist/`(该目录被
  gitignore;打包前校验 submodule 已初始化);
- `--target=<triple>`:交叉编译;`--install`:构建后安装;`--dry-run`:预览。
- 产物命名形如 `zhuye-cli-<version>-<triple>.tar.gz`。

`./scripts/install.sh`(全局安装):

- 优先安装已有 `target/release/zhuye-cli`(缺失自动构建);
- 默认 `/usr/local/bin`;`--local` → `~/.local/bin`;`--install-dir` 自定义。

## 5. 发布流水线(release.yml)

CI(`.github/workflows/release.yml`)仅在 `v*` tag 时于 macOS / Linux /
Windows 并行构建分发包,需递归 checkout submodule。发布用的是
`build.sh --dist`,它只负责构建与打包,**不含**测试 —— 所以审查必须先于
打 tag 完成。
