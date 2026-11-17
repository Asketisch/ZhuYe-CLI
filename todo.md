# ZhuYe-CLI · 剩余工作清单

> 基线:`develop` @ 7257b4b(F49 收口第二批)。todolist 引擎侧 + CLI 接线项已全部落地,
> 本文件记录 F49 的诚实标注剩余项与验证欠账。

## 一、F49 诚实标注剩余(主链路已收口,以下是深度增强项)

- [ ] **kitty graphics 完整编解码**:当前 Sixel 渲染为占位实现,kitty 协议编解码未做全。
- [ ] **并行实时双流分屏**:`/split` 已做快照版;子代理运行中的实时双流刷新未做。
- [ ] **看板 / attention zone**:多 agent 看板视图与注意力区域标注未实现。
- [ ] **screen-reader 深度项**:已做 env 开关 + 静态文本降级;完整无障碍语义树未做。

## 二、验证欠账

- [ ] **runner.py 四层选集完整重跑**:最后一批提交(5460a78 + 7257b4b)后,
      `pr / main / nightly / release` 选集未由主 agent 完整重跑;
      subagent 已验证 `make test-suite` 全绿(1481 lib + functional 218 + audit 86 命令)
      与 fmt/clippy——补跑 nightly / release 两层即可收口。

## 三、长期约束(非欠账,持续遵守)

- 嵌套 workspace:核心 crate 不入 `workspace.members`,根 `[workspace.dependencies]`
  需完整声明全部 zhuye-* 与外部依赖;升级 submodule 后同步补声明。
