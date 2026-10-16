pub mod agent;
pub mod boot_settings;
pub mod commands;
pub mod config;
pub mod gateway;
pub mod host;
pub mod main_host;
pub mod mcp_host;
pub mod repositories;
pub mod services;

#[cfg(not(test))]
use std::net::SocketAddr;

#[cfg(not(test))]
use tauri::Manager;

pub use host::{decide_spawn, wait_for_port, EmbeddedMcpRuntime, SpawnDecision};

pub const DEFAULT_MCP_PORT: u16 = config::settings::DEFAULT_PROD_MCP_PORT;
pub const AI_ASSISTANT_LABEL: &str = "ai-assistant";
pub const PLAN_ATTACHMENT_DIALOG_EXTENSIONS: &[&str] = &["md"];

#[cfg_attr(not(test), tauri::command)]
fn ping() -> &'static str {
    "pong"
}

#[cfg(not(test))]
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            ping,
            commands::read::search_spark,
            commands::read::get_annotations,
            commands::read::get_tags_registry,
            commands::read::get_annotation,
            commands::read::get_draft,
            commands::read::get_note_draft,
            commands::read::get_doc_highlights,
            commands::read::get_config,
            commands::config_cmd::set_config,
            commands::mcp_oauth::issue_cursor_ide_ticket,
            commands::mcp_oauth::rotate_cursor_ide_ticket,
            commands::mcp_oauth::revoke_mcp_slot_ticket,
            commands::mcp_oauth::get_mcp_ticket_view,
            commands::mcp_oauth::revoke_mcp_device_ticket,
            commands::mcp_channel_tools::get_mcp_channel_tools,
            commands::mcp_channel_tools::set_mcp_channel_tools,
            commands::bind::issue_bind,
            commands::bind::read_bind_session,
            commands::sync::save_comment_draft,
            commands::sync::save_note_draft,
            commands::sync::clear_note_draft,
            commands::sync::delete_entry,
            commands::sync::move_entry_project,
            commands::read::fetch_link_title,
            commands::read::get_notes_index,
            commands::read::get_notes_file,
            commands::read::resolve_note_for_open,
            commands::read::read_abs_file,
            commands::read::get_notes_asset,
            commands::workspaces::list_workspaces,
            commands::workspaces::create_workspace,
            commands::workspaces::rename_workspace,
            commands::workspaces::set_workspace_view_mode,
            commands::workspaces::delete_workspace,
            commands::workspaces::list_projects,
            commands::workspaces::create_project,
            commands::workspaces::update_project,
            commands::workspaces::delete_project,
            commands::read::get_message_channel_unread,
            commands::read_later::create_read_later,
            commands::read_later::get_read_later,
            commands::read_later::mark_read_later,
            commands::read_later::delete_read_later,
            commands::ai_assistant::present_ai_assistant,
            commands::ai_assistant::ensure_ai_assistant_session,
            commands::ai_assistant::shell_close_ai_assistant,
            commands::ai_assistant::get_ai_assistant_binding,
            commands::ai_assistant::list_chat_sessions,
            commands::ai_assistant::list_chat_workspace,
            commands::ai_assistant::delete_chat_workspace_file,
            commands::ai_assistant::select_chat_session,
            commands::ai_assistant::create_chat_session,
            commands::ai_assistant::delete_chat_session,
            commands::ai_assistant::unstage_chat_staged,
            commands::approval::resolve_approval,
            commands::ai_assistant::set_binding,
            commands::ai_assistant::reset_binding,
            commands::ai_assistant::defensive_unbound,
            commands::ai_assistant::query_binding,
            commands::ai_assistant::execute_binding,
            commands::ai_assistant::cancel_ai_assistant_turn,
            commands::ai_assistant::agent_chat_turn,
            commands::search::reindex_all,
            commands::search::get_reindex_all_status,
            commands::write::set_done,
            commands::write::set_importance,
            commands::write::update_links,
            commands::write::update_comments,
            commands::write::reorder_comments,
            commands::write::update_highlights,
            commands::write::update_doc_highlights,
            commands::write::save_entry,
            commands::write::write_abs_file,
            commands::write::tag_attach,
            commands::write::tag_detach,
            commands::write::tag_update_value,
            commands::write::create_note,
            commands::write::mark_message_channel_read,
            commands::os_notification::show_os_notification,
            commands::app_log::log_app_event,
            commands::auth_session::get_auth_session,
            commands::package::get_package_snapshot,
            commands::auth_session::set_auth_session,
            commands::auth_session::delete_auth_session,
        ])
        .setup(|app| {
            let notify_handle = app.handle().clone();
            crate::services::message_center::set_changed_handler(Some(std::sync::Arc::new(
                move || {
                    use tauri::Emitter;
                    if let Some(envelope) =
                        crate::services::message_center::last_changed_envelope()
                    {
                        let _ = notify_handle.emit("message-center:changed", envelope);
                    }
                },
            )));

            let main_host = crate::main_host::MainHostState::new();
            let mut embedded_mcp_handle = None;
            let boot = boot_settings::resolve_boot_settings()?;
            crate::services::app_log::init();
            crate::commands::approval::install_emitter(app.handle().clone());
            crate::commands::mcp_ide_env::apply_live_ide_ticket_envs();
            let (http_port, mcp_port) = (boot.http_port, boot.mcp_port);
            if let Ok(repo_root) = crate::config::paths::repo_root() {
                main_host.try_start(repo_root.clone(), http_port);
                // Embedded MCP only (dual-listen with Sidecar). Bind failure is fail-closed:
                // never fall back to spawning a Node MCP sidecar.
                let mcp_bind = SocketAddr::from(([127, 0, 0, 1], mcp_port));
                match crate::mcp_host::start_embedded_mcp_runtime(
                    crate::mcp_host::McpRuntimeConfig {
                        bind_addr: mcp_bind,
                    },
                ) {
                    Ok(handle) => {
                        embedded_mcp_handle = Some(handle);
                    }
                    Err(err) => {
                        eprintln!("[mcp-runtime] embedded start failed (fail-closed): {err}");
                    }
                }
            }
            let gateway = crate::gateway::GatewayState::new();
            let discovery = crate::gateway::discovery::DiscoveryState::new();
            if host::lan_ip::current_lan_ipv4().is_some() {
                if let Ok(config_dir) = config::settings::settings_config_dir() {
                    let gateway_port = boot.gateway_port;
                    match crate::gateway::boot(boot.gateway_port, boot.mcp_port, boot.http_port, config_dir) {
                        crate::gateway::BootDecision::Started(handle) => {
                            eprintln!("[gateway] listening 0.0.0.0:{gateway_port}");
                            gateway.set(handle);
                            discovery.try_start(gateway_port);
                        }
                        crate::gateway::BootDecision::SkippedNoLanIp => {}
                        crate::gateway::BootDecision::Failed(err) => {
                            eprintln!("[gateway] start failed: {err}");
                        }
                    }
                }
            }
            // L2 Host key→MCP registry: seed L1 internal MCP business surface before Binding Set.
            crate::mcp_host::seed_defaults();
            app.manage(EmbeddedMcpRuntime::new(embedded_mcp_handle));
            app.manage(main_host);
            app.manage(gateway);
            app.manage(discovery);

            host::create_main_window(app)?;
            app.manage(services::reindex::ReindexState::new());

            // Startup, in order: copy the legacy files into their workspaces (the index reads them),
            // rebuild the keyword index (index only, no git pull; the header does not show this
            // job), then back up the database. No Agent Loop warmup: the first chat pays cold start.
            {
                use tauri::Manager;
                let reindex = app.state::<services::reindex::ReindexState>();
                let spark_slot = std::sync::Arc::clone(&reindex.spark_job);
                std::thread::spawn(move || {
                    use crate::services::app_log::{log, Level, Side};
                    if let Ok(data) = crate::config::paths::runtime_data_dir() {
                        match crate::services::legacy_copy::copy_legacy_files_at_startup(&data) {
                            Ok(report) if report.already_done && report.removed == 0 => {}
                            Ok(report) => log(
                                "workspaces",
                                "legacy_copy_done",
                                None,
                                Some(serde_json::json!({
                                    "copied": report.copied,
                                    "skipped_existing": report.skipped_existing,
                                    "missing_folders": report.missing_folders,
                                    "removed": report.removed,
                                })),
                                Side::Host,
                                Level::Info,
                            ),
                            Err(error) => log(
                                "workspaces",
                                "legacy_copy_failed",
                                None,
                                Some(serde_json::json!({ "error": error })),
                                Side::Host,
                                Level::Error,
                            ),
                        }
                    }
                    if let Ok(repo_root) = crate::config::paths::repo_root() {
                        let _ = services::reindex::run_all_reindex_blocking(&repo_root, &spark_slot);
                    }
                    if let Err(error) = crate::services::spark_data::backup_daily() {
                        log(
                            "spark_data",
                            "backup_failed",
                            None,
                            Some(serde_json::json!({ "error": error })),
                            Side::Host,
                            Level::Error,
                        );
                    }
                });
            }

            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app_handle, event| {
            match event {
                tauri::RunEvent::Opened { urls } => {
                    for url in urls {
                        crate::services::os_notify_trace::log_scheme_open(url.as_str());
                        crate::commands::auth_session::emit_opened_scheme(app_handle, url.as_str());
                        if crate::services::login_hop::is_auth_login_scheme(url.as_str()) {
                            crate::host::present_main_window(app_handle);
                        }
                    }
                }
                tauri::RunEvent::Exit => {
                    crate::services::app_log::shutdown();
                    // Before the embedded MCP server stops, so the session's
                    // DELETE still reaches it.
                    crate::agent::mcp::release_connection(crate::agent::mcp::ResetReason::AppExit);
                    if let Some(embedded) = app_handle.try_state::<EmbeddedMcpRuntime>() {
                        embedded.stop();
                    }
                    if let Some(main_host) =
                        app_handle.try_state::<crate::main_host::MainHostState>()
                    {
                        main_host.stop();
                    }
                    if let Some(discovery) =
                        app_handle.try_state::<crate::gateway::discovery::DiscoveryState>()
                    {
                        discovery.stop();
                    }
                    if let Some(gateway) = app_handle.try_state::<crate::gateway::GatewayState>() {
                        gateway.stop();
                    }
                }
                _ => {}
            }
        });
}

#[cfg(test)]
mod test_support;

#[cfg(test)]
#[path = "unit-tests/lib/mod.rs"]
mod lib_tests;
