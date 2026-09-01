pub mod commands;
pub mod config;
pub mod host;
pub mod integrations;
pub mod repositories;
pub mod services;

use std::net::SocketAddr;

#[cfg(not(test))]
use tauri::Manager;

pub use host::{decide_spawn, wait_for_port, EmbeddedMcpRuntime, SpawnDecision};

pub const DEFAULT_MCP_PORT: u16 = config::settings::DEFAULT_PROD_MCP_PORT;
pub const AI_ASSISTANT_LABEL: &str = "ai-assistant";
pub const PLAN_ATTACHMENT_DIALOG_EXTENSIONS: &[&str] = &["md"];

#[cfg(not(test))]
#[tauri::command]
fn ping() -> &'static str {
    "pong"
}

#[cfg(test)]
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
            commands::read::search_knowledge,
            commands::read::search_workbench,
            commands::read::get_topics,
            commands::read::get_annotations,
            commands::read::get_tags_registry,
            commands::read::get_annotation,
            commands::read::get_draft,
            commands::read::get_note_draft,
            commands::read::get_doc_highlights,
            commands::read::get_config,
            commands::read::infer_github_user_url,
            commands::read::check_workbench_root,
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
            commands::sync::workbench_git_commit,
            commands::sync::workbench_git_pull,
            commands::sync::workbench_git_revert,
            commands::sync::kb_git_commit,
            commands::sync::kb_git_revert,
            commands::sync::delete_entry,
            commands::sync::move_entry_project,
            commands::sync::gh_move_assets,
            commands::sync::gh_delete_assets,
            commands::sync::settle_entry,
            commands::sync::open_kb_in_iterm,
            commands::read::get_status,
            commands::read::kb_read,
            commands::read::kb_list,
            commands::read::kb_doc_count,
            commands::read::kb_annotation,
            commands::read::kb_status,
            commands::read::get_repo_dirs,
            commands::read::check_file,
            commands::read::fetch_link_title,
            commands::read::get_notes_index,
            commands::read::get_notes_file,
            commands::read::get_notes_asset,
            commands::notes_categories::list_notes_categories,
            commands::notes_categories::create_notes_category,
            commands::notes_categories::update_notes_category,
            commands::notes_categories::delete_notes_category,
            commands::read::get_kb_diff_status,
            commands::read::get_sediment_kb_categories,
            commands::read::get_sediment_kb_repos,
            commands::read_later::create_read_later,
            commands::read_later::get_read_later,
            commands::read_later::mark_read_later,
            commands::read_later::delete_read_later,
            commands::todo_task::get_todo_tasks,
            commands::todo_task::create_todo_task,
            commands::todo_task::delete_todo_task,
            commands::todo_task::add_todo_sub,
            commands::todo_task::delete_todo_sub,
            commands::todo_task::read_todo_md,
            commands::todo_task::update_todo_md,
            commands::todo_task::complete_todo,
            commands::todo_task::abandon_todo_sub,
            commands::todo_task::update_todo_sub,
            commands::todo_task::update_todo_master_title,
            commands::todo_task::set_todo_master_status,
            commands::todo_task::stage_todo_attachment_source,
            commands::todo_task::add_todo_attachment,
            commands::todo_task::list_todo_attachments,
            commands::todo_task::read_todo_attachment,
            commands::todo_task::save_todo_attachment,
            commands::todo_task::delete_todo_attachment,
            commands::todo_task::list_todo_comments,
            commands::todo_task::add_todo_comment,
            commands::todo_task::update_todo_comment,
            commands::todo_task::delete_todo_comment,
            commands::todo_task::list_todo_categories,
            commands::todo_task::create_todo_category,
            commands::todo_task::delete_todo_category,
            commands::todo_task::set_todo_category,
            commands::ai_assistant::open_ai_assistant,
            commands::ai_assistant::present_ai_assistant,
            commands::ai_assistant::ensure_ai_assistant_session,
            commands::ai_assistant::shell_close_ai_assistant,
            commands::ai_assistant::get_ai_assistant_binding,
            commands::ai_assistant::list_chat_sessions,
            commands::ai_assistant::select_chat_session,
            commands::ai_assistant::create_chat_session,
            commands::ai_assistant::delete_chat_session,
            commands::ai_assistant::unstage_chat_staged,
            commands::ai_assistant::set_binding,
            commands::ai_assistant::reset_binding,
            commands::ai_assistant::defensive_unbound,
            commands::ai_assistant::query_binding,
            commands::ai_assistant::execute_binding,
            commands::ai_assistant::cancel_ai_assistant_turn,
            commands::ai_assistant::agent_chat_turn,
            commands::search::reindex_knowledge,
            commands::search::reindex_workbench,
            commands::search::reindex_kb_repo,
            commands::search::sync_knowledge,
            commands::search::get_reindex_status,
            commands::search::get_reindex_workbench_status,
            commands::write::set_done,
            commands::write::set_importance,
            commands::write::update_links,
            commands::write::update_comments,
            commands::write::reorder_comments,
            commands::write::update_highlights,
            commands::write::update_doc_highlights,
            commands::write::save_entry,
            commands::write::kb_save,
            commands::write::kb_update_comments,
            commands::write::kb_reorder_comments,
            commands::write::kb_update_highlights,
            commands::write::kb_update_links,
            commands::write::tag_attach,
            commands::write::tag_detach,
            commands::write::tag_update_value,
            commands::write::sediment_kb_add_repo,
            commands::write::sediment_kb_remove_repo,
            commands::write::sediment_kb_update_repo_category,
            commands::write::sediment_kb_add_category,
            commands::write::sediment_kb_rename_category,
            commands::write::sediment_kb_remove_category,
            commands::write::create_note,
        ])
        .setup(|app| {
            // Auto-start Meilisearch if not already running
            let meili_child = host::try_autostart_meilisearch();
            app.manage(host::MeiliProcess::new(meili_child));

            let local_http = services::local_http::LocalHttpState::new();
            let mut embedded_mcp_handle = None;
            let boot_settings = match config::settings::load() {
                Ok(s) => s,
                Err(err) => {
                    eprintln!("[settings] load failed: {err}");
                    if config::settings::is_test_sandbox() {
                        return Err(format!("sandbox settings: {err}").into());
                    }
                    config::settings::AppSettings::default()
                }
            };
            let http_port = boot_settings.effective_http_port();
            let mcp_port = boot_settings.effective_mcp_port();
            if let Ok(repo_root) = crate::config::paths::repo_root() {
                local_http.try_start(repo_root.clone(), http_port);
                // Embedded MCP only (dual-listen with Sidecar). Bind failure is fail-closed:
                // never fall back to spawning a Node MCP sidecar.
                let mcp_bind = SocketAddr::from(([127, 0, 0, 1], mcp_port));
                match services::mcp_host::start_embedded_mcp_runtime(
                    services::mcp_host::McpRuntimeConfig {
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
            let gateway = services::gateway::GatewayState::new();
            let discovery = services::discovery::DiscoveryState::new();
            if services::lan_ip::current_lan_ipv4().is_some() {
                if let Ok(config_dir) = config::settings::settings_config_dir() {
                    let gateway_port = boot_settings.effective_gateway_port();
                    match services::gateway::boot(&boot_settings, config_dir) {
                        services::gateway::BootDecision::Started(handle) => {
                            eprintln!("[gateway] listening 0.0.0.0:{gateway_port}");
                            gateway.set(handle);
                            discovery.try_start(gateway_port);
                        }
                        services::gateway::BootDecision::SkippedNoLanIp => {}
                        services::gateway::BootDecision::Failed(err) => {
                            eprintln!("[gateway] start failed: {err}");
                        }
                    }
                }
            }
            // L2 Host key→MCP registry: seed L1 internal MCP business surface before Binding Set.
            services::mcp_host::seed_defaults();
            app.manage(EmbeddedMcpRuntime::new(embedded_mcp_handle));
            app.manage(local_http);
            app.manage(gateway);
            app.manage(discovery);

            host::create_main_window(app)?;
            app.manage(services::reindex::ReindexState::new());

            // Phase-Warm: retained coordination seam; Agent Loop warmup is deferred.
            services::agent::host_startup::schedule_agent_loop_warm();

            let app_handle = app.handle().clone();
            std::thread::spawn(move || {
                let Ok(repo_root) = crate::config::paths::repo_root() else {
                    return;
                };
                let wb = crate::config::meili_env::workbench_root_path(&repo_root);
                let _ = crate::services::notes::ensure_notes_layout(&wb);
                let _ = crate::services::knowledge_layout::ensure_knowledge_registry_layout(&wb);
                if !wb.join("notes").join("index.json").is_file() {
                    return;
                }
                if services::tags_registry::reconcile_tags(&repo_root).is_none() {
                    use tauri::Emitter;
                    let _ = app_handle.emit("tags:reconciled", ());
                }
            });

            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app_handle, event| {
            if let tauri::RunEvent::Exit = event {
                if let Some(meili) = app_handle.try_state::<host::MeiliProcess>() {
                    meili.kill();
                }
                if let Some(embedded) = app_handle.try_state::<EmbeddedMcpRuntime>() {
                    embedded.stop();
                }
                if let Some(local_http) = app_handle.try_state::<services::local_http::LocalHttpState>()
                {
                    local_http.stop();
                }
                if let Some(discovery) = app_handle.try_state::<services::discovery::DiscoveryState>()
                {
                    discovery.stop();
                }
                if let Some(gateway) = app_handle.try_state::<services::gateway::GatewayState>() {
                    gateway.stop();
                }
            }
        });
}

#[cfg(test)]
mod test_support;

#[cfg(test)]
#[path = "unit-tests/test_support.rs"]
mod test_support_tests;

#[cfg(test)]
#[path = "unit-tests/lib/wait_for_port_tests.rs"]
mod wait_for_port_tests;

#[cfg(test)]
#[path = "unit-tests/lib/ping_tests.rs"]
mod ping_tests;

#[cfg(test)]
#[path = "unit-tests/lib/spawn_decision_tests.rs"]
mod spawn_decision_tests;

#[cfg(test)]
#[path = "unit-tests/lib/knowledge_mcp_tests.rs"]
mod knowledge_mcp_tests;

#[cfg(test)]
#[path = "unit-tests/lib/read_later_assistant_window_tests.rs"]
mod read_later_assistant_window_tests;

#[cfg(test)]
#[path = "unit-tests/lib/ai_assistant_window_tests.rs"]
mod ai_assistant_window_tests;

#[cfg(test)]
#[path = "unit-tests/lib/gateway_startup_tests.rs"]
mod gateway_startup_tests;

#[cfg(test)]
#[path = "unit-tests/lib/discovery_startup_tests.rs"]
mod discovery_startup_tests;
