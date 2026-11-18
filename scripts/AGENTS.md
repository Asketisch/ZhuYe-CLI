# AGENTS.md — scripts/(构建 / 发版 / 安装脚本)

> **何时读我**:改 `scripts/` 下任何脚本、调整构建打包 / 发版 tag / 安装行为之前,先读本篇。

## 脚本速览(共 3 个可执行脚本)

| 脚本 | 用途 | 关键行为 |
|---|---|---|
| `build.sh` | 跨平台构建 | `--fast`(release-fast)/ `--install` / `--dist`(打包 tar.gz / zip 到 `dist/`)/ `--target=<triple>` 交叉编译 / `--dry-run`;**打包前校验 submodule 已初始化** |
| `create_tag.sh` | 一站式发版 tag | 版本 bump + CHANGELOG 转正 + 提交 + annotated tag + 推送;`--dry-run` / `--no-push` / `--yes`;**推送 `v*` tag 触发 `.github/workflows/release.yml`** |
| `install.sh` | 全局安装 | 优先装已有 `target/release/zhuye-cli`(缺失自动构建);默认 `/usr/local/bin`,`--local` → `~/.local/bin`,可 `--install-dir` 自定义 |

`dist/` 产物目录被 gitignore;Makefile 侧入口见根 AGENTS.md 常用命令
(`make dist` / `make dist-dry` / `make install`)。

## 本地契约 / 同步点

- 新增 / 移除脚本或改变既有 flag 语义 → 更新本篇速览表,并核对根 AGENTS.md
  「常用命令」与 README 的构建发布章节是否需要联动。
- 发版链路:`create_tag.sh` 的版本 bump 与 `CHANGELOG.md` 转正必须同一次执行完成;
  只想验证流程用 `--dry-run`,**测试环境不推真 tag**。

## 注意事项

- 产物命名形如 `zhuye-cli-<version>-<triple>.tar.gz`(见 `dist/` 既有示例),
  压缩格式按平台取 tar.gz / zip,以 `build.sh` 实现为准。
- 验证打包 / 发版命令序列优先用 `--dry-run`(只打印不执行)。

## 禁止事项

- 不新增会隐式改宿主状态的步骤(`npm install` / `cargo install` / 改全局配置),
  确需安装依赖时显式打印并给出退出途径。
- 不绕过 `build.sh --dist` 的 submodule 校验逻辑硬打包。

## 深入阅读

- [CONTRIBUTING.md](../CONTRIBUTING.md)(发布前审查)· `.github/workflows/release.yml`(tag 触发的产物流水线)
