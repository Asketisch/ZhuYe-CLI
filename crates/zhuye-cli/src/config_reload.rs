//! 配置热重载接线(v1.x `ConfigWatcher`)。
//!
//! 背景:submodule 的 `zhuye_config::ConfigWatcher` 会监听
//! `~/.zhuye/config.toml` 的变更并把新配置推给订阅者;headless
//! (`zhuye-exec`)据此重建 provider client、切换 model、推
//! `EventMsg::ConfigReloaded`。TUI 此前**完全没有**这个 watcher ——
//! 改配置后必须重启 TUI 才生效,`[routing] fallbacks`、`[model]` 之类
//! 的改动统统不认。
//!
//! 本模块负责:
//! - `diff_sections` —— 纯函数,算出本轮变更了哪些配置段;
//! - `spawn_config_reload_task` —— 后台任务:应用新 provider client →
//!   必要时切 model → 推一条 Notice 回界面。

use std::sync::Arc;

use zhuye_config::ZhuyeConfig;
use zhuye_core::AgentConfig;
use zhuye_llm::ModelRegistry;

/// 算出两份配置之间变更了哪些段(段名列表)。
///
/// 段名与 `zhuye-exec` 的 `diff_sections` 对齐,便于两边行为一致。
pub fn diff_sections(old: &ZhuyeConfig, new: &ZhuyeConfig) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    macro_rules! cmp_section {
        ($field:ident, $name:literal) => {
            if old.$field != new.$field {
                out.push($name.to_string());
            }
        };
    }
    cmp_section!(active, "active");
    cmp_section!(prompts, "prompts");
    // 模型组(`[[model_group]]`)是 v2 唯一的模型接入单元 —— 组增删改
    // (端点 / 凭据 / 默认 model)统一报 "model_group",与引擎 exec 的
    // diff_sections 同名对齐(上游 8da90e5 起)。registry 侧经
    // `apply_to_registry` 已生效,此处只补段名报告。
    cmp_section!(model_group, "model_group");
    cmp_section!(compact, "compact");
    cmp_section!(token_budget, "token_budget");
    cmp_section!(sandbox, "sandbox");
    cmp_section!(hooks, "hooks");
    cmp_section!(mcp_servers, "mcp_servers");
    cmp_section!(lsp_servers, "lsp_servers");
    cmp_section!(plugins, "plugins");
    cmp_section!(routing, "routing");
    cmp_section!(subagent_models, "subagent_models");
    cmp_section!(coordinator, "coordinator");
    cmp_section!(sanitize, "sanitize");
    cmp_section!(model, "model");
    cmp_section!(model_discovery, "model_discovery");
    cmp_section!(context_windows, "context_windows");
    cmp_section!(goal, "goal");
    cmp_section!(telemetry, "telemetry");
    cmp_section!(analytics, "analytics");
    cmp_section!(notifications, "notifications");
    // v1.x:`[cache_warming]` 段变更 → reload 报告(与引擎 exec 的
    // diff_sections 名单对齐;关闭即取消在飞保温任务,用户应看到反馈)。
    cmp_section!(cache_warming, "cache_warming");
    cmp_section!(web_search, "web_search");
    // v1.x(e52bfda 起)新增段:与 bootstrap_wiring::apply_feature_sections
    // 的装配面同批 —— 段改了热重载要如实报告(实际生效发生在下一进程
    // 实例或下一回合,视段而定)。
    cmp_section!(task_budget, "task_budget");
    cmp_section!(provenance, "provenance");
    cmp_section!(dynamic_context, "dynamic_context");
    cmp_section!(tool_journal, "tool_journal");
    cmp_section!(turn_scheduling, "turn_scheduling");
    // 存量补漏:`[tool_results]` 段(e246299 起已在 schema,名单一直没跟上)。
    cmp_section!(tool_results, "tool_results");
    cmp_section!(permissions, "permissions");
    // v2 新增段:TUI 补全版比引擎 exec 的名单更全(exec 只报常用段),
    // 这里保持「能 diff 的段都报」口径,Notice 不漏变更。
    cmp_section!(ask_user_question, "ask_user_question");
    cmp_section!(prompt_guard, "prompt_guard");
    cmp_section!(subagents, "subagents");
    cmp_section!(postgres_session, "postgres_session");
    cmp_section!(sse_redis, "sse_redis");
    cmp_section!(bridge, "bridge");
    cmp_section!(voice, "voice");
    cmp_section!(dap, "dap");
    cmp_section!(acp, "acp");
    out
}

/// 启动配置热重载后台任务。
///
/// 每次配置变更:
/// 1. `apply_to_registry` 替换 provider client(失败 → 保留旧 provider,
///    只 warn,不推事件 —— 与 headless 同款语义);
/// 2. model spec 变了 → `agent_cfg.set_model(...)` 同步给运行中的 agent;
/// 3. 推一条 Notice 回界面,告诉用户「配置已重载,哪些段变了」。
///
/// `ui_tx` 是 TUI 的 `UiEvent` 通道 —— 直接推 `UiEventKind::Notice`,
/// 免得绕协议事件再转一次。
pub fn spawn_config_reload_task(
    watcher: zhuye_config::ConfigWatcher,
    registry: Arc<ModelRegistry>,
    agent_cfg: AgentConfig,
    path: std::path::PathBuf,
    previous_cfg: ZhuyeConfig,
    ui_tx: tokio::sync::mpsc::UnboundedSender<crate::adapter::UiEvent>,
) {
    tokio::spawn(async move {
        let mut rx = watcher.subscribe();
        // 不要在循环前预调 `rx.changed()`「跳过初始值」—— watch::Receiver
        // 新订阅时已把当前值视为已见,那次预调会吞掉启动后第一次真实变更。
        let mut prev = previous_cfg;
        while rx.changed().await.is_ok() {
            let new_cfg = rx.borrow().clone();
            match handle_reload(&prev, &new_cfg, &registry, &agent_cfg, &path, &ui_tx) {
                Ok(()) => {}
                Err(e) => tracing::warn!(error = %e, "config reload failed"),
            }
            prev = new_cfg;
        }
        tracing::debug!("config reload task exiting");
    });
}

/// 应用一次配置变更。返回 `Ok(())` 表示已生效并已推事件。
fn handle_reload(
    old_cfg: &ZhuyeConfig,
    new_cfg: &ZhuyeConfig,
    registry: &Arc<ModelRegistry>,
    agent_cfg: &AgentConfig,
    path: &std::path::Path,
    ui_tx: &tokio::sync::mpsc::UnboundedSender<crate::adapter::UiEvent>,
) -> Result<(), String> {
    // provider client 重建失败 → 保留旧 provider,不推事件(与 headless 一致)。
    new_cfg
        .apply_to_registry(registry)
        .map_err(|e| format!("apply_to_registry failed: {e}"))?;
    let sections_changed = diff_sections(old_cfg, new_cfg);
    if old_cfg.prompts != new_cfg.prompts {
        *agent_cfg.prompt_default.write() = new_cfg.prompts.clone();
    }
    // model spec 变了 → 同步给运行中的 agent(下一轮生效)。
    if let Some(spec) = new_cfg.resolved_model_spec()
        && old_cfg.resolved_model_spec().as_deref() != Some(spec.as_str())
    {
        agent_cfg.set_model(spec.clone());
        tracing::info!(model = %spec, "config reload: model switched");
    }
    tracing::info!(path = %path.display(), "config reload applied");
    // 推给界面(通道关闭时忽略 —— TUI 已退出)。事件携带新配置,消费方
    // 刷新 `state.zhuye_config` 并渲染变更提示。
    let _ = ui_tx.send(crate::adapter::UiEvent {
        kind: crate::adapter::UiEventKind::ConfigReloaded {
            config: Box::new(new_cfg.clone()),
            sections_changed,
        },
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg_with_model(model: &str) -> ZhuyeConfig {
        ZhuyeConfig {
            model_group: vec![zhuye_config::ModelGroupSection {
                id: "openai".into(),
                endpoint: "openai".into(),
                api_key: Some("k".into()),
                model: Some(model.into()),
                ..Default::default()
            }],
            ..Default::default()
        }
    }

    #[test]
    fn no_change_yields_empty_diff() {
        let a = cfg_with_model("gpt-4o");
        let b = cfg_with_model("gpt-4o");
        assert!(diff_sections(&a, &b).is_empty());
    }

    #[test]
    fn model_change_is_reported() {
        let a = cfg_with_model("gpt-4o");
        let b = cfg_with_model("gpt-4o-mini");
        let d = diff_sections(&a, &b);
        // v2:model 挂在模型组(`[[model_group]]` 条目的 `model` 字段),
        // `[active]` 只剩 group / credential / max_iterations 等 —— 组没变
        // 则 active 不变。
        assert!(d.iter().any(|s| s == "model_group"), "{d:?}");
        assert!(!d.iter().any(|s| s == "active"), "{d:?}");
    }

    #[test]
    fn routing_change_is_reported() {
        let a = ZhuyeConfig::default();
        let mut b = ZhuyeConfig::default();
        b.routing = Some(zhuye_config::RoutingSection::default());
        let d = diff_sections(&a, &b);
        assert!(d.iter().any(|s| s == "routing"), "{d:?}");
    }

    #[test]
    fn model_discovery_change_is_reported() {
        let a = ZhuyeConfig::default();
        let mut b = ZhuyeConfig::default();
        b.model_discovery = Some(zhuye_config::ModelDiscoverySection::default());
        let d = diff_sections(&a, &b);
        assert!(d.iter().any(|s| s == "model_discovery"), "{d:?}");
    }

    #[test]
    fn model_group_change_is_reported() {
        // `[[model_group]]` 段增删改都要进热重载 Notice 的变更段列表。
        let a = ZhuyeConfig::default();
        let mut b = ZhuyeConfig::default();
        b.model_group = vec![zhuye_config::ModelGroupSection {
            id: "bigmodel".into(),
            endpoint: "openai".into(),
            api_key: Some("k".into()),
            ..Default::default()
        }];
        let d = diff_sections(&a, &b);
        assert!(d.iter().any(|s| s == "model_group"), "{d:?}");
    }

    #[test]
    fn subagent_models_change_is_reported() {
        // v2 段名 `[subagent_models]`(v1 旧名 `[subagent_providers]` 由
        // 引擎 load 迁移接管)—— 段变更要进 Notice 列表。
        let a = ZhuyeConfig::default();
        let mut b = ZhuyeConfig::default();
        b.subagent_models = Some(zhuye_config::schema::SubagentModelsSection::default());
        let d = diff_sections(&a, &b);
        assert!(d.iter().any(|s| s == "subagent_models"), "{d:?}");
    }

    #[test]
    fn cache_warming_change_is_reported() {
        // v1.x:`[cache_warming]` 段变更要进 Notice 列表(与引擎 exec 的
        // diff_sections 名单对齐;此前漏段比,Notice 报「变更段: (无)」)。
        let a = ZhuyeConfig::default();
        let mut b = ZhuyeConfig::default();
        b.cache_warming = Some(zhuye_config::schema::CacheWarmingSection {
            enabled: Some(false),
            ..Default::default()
        });
        let d = diff_sections(&a, &b);
        assert!(d.iter().any(|s| s == "cache_warming"), "{d:?}");
    }

    #[test]
    fn v1x_feature_sections_change_is_reported() {
        // v1.x(e52bfda 起)新增的五个特性段 + 存量 `[tool_results]`,
        // 段变更都要进热重载 Notice 列表(与 apply_feature_sections
        // 装配面同批,名单与 schema 顶层段对齐)。
        let a = ZhuyeConfig::default();
        let mut b = ZhuyeConfig::default();
        b.task_budget = Some(zhuye_config::schema::TaskBudgetSection::default());
        b.provenance = Some(zhuye_config::schema::ProvenanceSection::default());
        b.dynamic_context = Some(zhuye_config::schema::DynamicContextSection::default());
        b.tool_journal = Some(zhuye_config::schema::ToolJournalSection::default());
        b.turn_scheduling = Some(zhuye_config::schema::TurnSchedulingSection::default());
        b.tool_results = Some(zhuye_config::schema::ToolResultsSection::default());
        let d = diff_sections(&a, &b);
        for name in [
            "task_budget",
            "provenance",
            "dynamic_context",
            "tool_journal",
            "turn_scheduling",
            "tool_results",
        ] {
            assert!(d.iter().any(|s| s == name), "缺 {name}:{d:?}");
        }
    }
}
