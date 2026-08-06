//! Loop + Host open/chat-turn contract tests (t4 / t3 Host empty-tools).

use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use serde_json::{json, Value};

use crate::config::secrets::{self, KEY_LLM_API_KEY};
use crate::config::settings;
use crate::services::agent::llm::LlmConfig;
use crate::services::agent::r#loop::{self, Terminal, TurnOutcome, EVENT_TURN_COMPLETED};
use crate::services::agent::session::{self, Turn};
use crate::services::agent::PLAN_ASSISTANT_SYSTEM_PROMPT;
use crate::services::todo_task;
use crate::test_support::TestSandbox;

fn with_sandbox<F: FnOnce()>(f: F) {
    let _sandbox = TestSandbox::new();
    secrets::test_secrets_clear();
    r#loop::reset_runtime_for_tests();
    crate::services::mcp_server_registry::clear_for_tests();
    crate::services::mcp_server_registry::seed_defaults();
    f();
}

fn create_bound_plan(title: &str) -> String {
    let created = todo_task::create_master_with_subs(title, Some(&["子项A"]));
    assert_eq!(created["_status"], 201);
    created["master_task_id"].as_str().unwrap().to_string()
}


fn plan_tools_binding(master: &str) -> r#loop::Binding {
    let tools = json!([
        { "name": "get_plan", "ctx": { "master_task_id": master } },
        { "name": "list_sub_tasks", "ctx": { "master_task_id": master } },
        { "name": "add_sub_task", "ctx": { "master_task_id": master } },
        { "name": "update_sub_title", "ctx": { "master_task_id": master } },
        { "name": "update_master_title", "ctx": { "master_task_id": master } },
    ]);
    r#loop::Binding {
        tools,
        prompt: json!(PLAN_ASSISTANT_SYSTEM_PROMPT),
        callbacks: json!({}),
    }
}

fn arm_plan_binding(master: &str) {
    r#loop::set_binding(plan_tools_binding(master)).expect("Set plan Binding");
}


struct MockLlm {
    port: u16,
    hits: Arc<Mutex<Vec<Value>>>,
    _join: thread::JoinHandle<()>,
}

fn spawn_scripted_llm(responses: Vec<(u16, Value)>) -> MockLlm {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().unwrap().port();
    let hits = Arc::new(Mutex::new(Vec::new()));
    let hits_t = hits.clone();
    let join = thread::spawn(move || {
        let mut idx = 0usize;
        for _ in 0..responses.len().saturating_add(4) {
            let Ok((mut stream, _)) = listener.accept() else {
                break;
            };
            let mut buf = [0u8; 65536];
            let n = stream.read(&mut buf).unwrap_or(0);
            let raw = String::from_utf8_lossy(&buf[..n]);
            let body_str = raw.split("\r\n\r\n").nth(1).unwrap_or("");
            let body: Value = serde_json::from_str(body_str.trim_end_matches('\0').trim())
                .unwrap_or(json!({}));
            hits_t.lock().unwrap().push(body);
            let (status, resp) = if idx < responses.len() {
                responses[idx].clone()
            } else {
                (
                    200,
                    json!({
                        "choices": [{
                            "finish_reason": "stop",
                            "message": { "role": "assistant", "content": "ok" }
                        }]
                    }),
                )
            };
            idx += 1;
            let body = resp.to_string();
            let resp = format!(
                "HTTP/1.1 {status} OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            let _ = stream.write_all(resp.as_bytes());
        }
    });
    thread::sleep(Duration::from_millis(20));
    MockLlm {
        port,
        hits,
        _join: join,
    }
}

fn cfg_for(mock: &MockLlm) -> LlmConfig {
    LlmConfig {
        api_key: "sk-test".into(),
        base_url: format!("http://127.0.0.1:{}", mock.port),
        model: "test-model".into(),
    }
}

fn install_llm_cfg(mock: &MockLlm) {
    let mut s = settings::load().expect("load");
    settings::apply_config_payload(
        &mut s,
        &json!({
            "assistant_engine": "host",
            "llm": {
                "model": "test-model"
            }
        }),
    )
    .expect("apply");
    // Preset platform/base_url are not client-writable via apply_config_payload;
    // persist mock URL on the settings struct for Host load_llm_config (same pattern as
    // unit-tests/services/agent/mod.rs::llm_load_config_reads_settings_and_secret).
    let model = settings::llm_entry_by_type(&s.llm, "host")
        .map(|e| e.model.clone())
        .unwrap_or_else(|| "test-model".into());
    settings::upsert_llm_entry(
        &mut s.llm,
        "host",
        &settings::LlmSettings {
            platform: "openai_compatible".into(),
            base_url: format!("http://127.0.0.1:{}", mock.port),
            model,
        },
    )
    .expect("upsert host llm");
    settings::save(&s).expect("save");
    secrets::set_secret(KEY_LLM_API_KEY, "sk-test").expect("key");
}

fn assistant_text(content: &str) -> (u16, Value) {
    (
        200,
        json!({
            "choices": [{
                "finish_reason": "stop",
                "message": { "role": "assistant", "content": content }
            }]
        }),
    )
}

fn assistant_tools(calls: Value, content: Option<&str>) -> (u16, Value) {
    (
        200,
        json!({
            "choices": [{
                "finish_reason": "tool_calls",
                "message": {
                    "role": "assistant",
                    "content": content,
                    "tool_calls": calls
                }
            }]
        }),
    )
}

/// Host business path: request must not carry a non-empty tools list.
fn assert_host_llm_tools_empty(body: &Value) {
    match body.get("tools") {
        None => {}
        Some(Value::Array(arr)) => {
            assert!(
                arr.is_empty(),
                "Host path must send tools=[] (empty), got {arr:?}"
            );
        }
        Some(other) => panic!("Host path tools must be absent or [], got {other}"),
    }
    if let Some(tc) = body.get("tool_choice") {
        // tool_choice without tools is not Host empty-tools semantics.
        panic!("Host empty-tools path must not send tool_choice, got {tc}");
    }
}

fn assert_outcome(o: &TurnOutcome, terminal: &str, wrote: bool) {
    assert_eq!(o.terminal.as_str(), terminal, "terminal mismatch: {o:?}");
    assert_eq!(o.wrote, wrote, "wrote mismatch: {o:?}");
    assert!(!o.reply_text.is_empty(), "reply_text empty: {o:?}");
}

#[test]
fn plan_assistant_system_prompt_is_nonempty_code_constant() {
    assert!(!PLAN_ASSISTANT_SYSTEM_PROMPT.trim().is_empty());
    assert!(PLAN_ASSISTANT_SYSTEM_PROMPT.contains("只服务"));
    assert!(PLAN_ASSISTANT_SYSTEM_PROMPT.contains("只读工具"));
    assert!(PLAN_ASSISTANT_SYSTEM_PROMPT.contains("目前不支持"));
    assert!(PLAN_ASSISTANT_SYSTEM_PROMPT.contains("API Key"));
}

#[test]
fn history_truncation_keeps_system_and_dual_hard_caps() {
    with_sandbox(|| {
        let mut turns = Vec::new();
        for i in 0..12 {
            turns.push(Turn {
                role: "user".into(),
                content: Some(format!("u{i}")),
                tool_call_id: None,
                tool_calls: None,
                name: None,
            });
            turns.push(Turn {
                role: "assistant".into(),
                content: Some(format!("a{i}")),
                tool_call_id: None,
                tool_calls: None,
                name: None,
            });
        }
        let messages = r#loop::build_llm_messages_from_turns(&turns, PLAN_ASSISTANT_SYSTEM_PROMPT);
        assert_eq!(messages[0]["role"], "system");
        assert_eq!(
            messages[0]["content"].as_str().unwrap(),
            PLAN_ASSISTANT_SYSTEM_PROMPT
        );
        assert!(messages.len() <= 21, "len={}", messages.len());
        let user_count = messages.iter().filter(|m| m["role"] == "user").count();
        assert!(user_count <= 8, "user_count={user_count}");
        let blob = serde_json::to_string(&messages).unwrap();
        assert!(!blob.contains("\"u0\""), "oldest user round should be dropped");
        assert!(blob.contains("\"u11\""), "newest user round should remain");
    });
}

#[test]
fn run_loop_final_reply_none_terminal_and_wrote_false() {
    with_sandbox(|| {
        let mock = spawn_scripted_llm(vec![assistant_text("你好，我是计划助手")]);
        let master = create_bound_plan("主计划");
        let mut sess = session::create_session(Some(&master), Some("主计划")).unwrap();
        arm_plan_binding(&master);
        let out = r#loop::run_loop(&mut sess, "你好", &cfg_for(&mock));
        assert_outcome(&out, "none", false);
        assert!(out.reply_text.contains("计划助手"));
        assert_eq!(sess.turns[0].role, "user");
        assert_eq!(sess.turns[1].role, "assistant");
        assert_host_llm_tools_empty(&mock.hits.lock().unwrap()[0]);
    });
}

#[test]
fn run_loop_host_empty_tools_rejects_tool_calls_without_dispatch() {
    // Narrowed from process-local tool write path (P3 / T3): Host business chat
    // sends tools=[] and must not tools::dispatch even if the model returns tool_calls.
    with_sandbox(|| {
        let master = create_bound_plan("写前标题");
        let title_before = todo_task::get_by_id(&master)["title"].clone();
        let mock = spawn_scripted_llm(vec![assistant_tools(
            json!([{
                "id": "call_1",
                "type": "function",
                "function": {
                    "name": "update_master_title",
                    "arguments": "{\"title\":\"写后标题\"}"
                }
            }]),
            None,
        )]);
        let mut sess = session::create_session(Some(&master), Some("写前标题")).unwrap();
        arm_plan_binding(&master);
        let out = r#loop::run_loop(&mut sess, "把主标题改成写后标题", &cfg_for(&mock));
        assert_outcome(&out, "error", false);
        assert_eq!(todo_task::get_by_id(&master)["title"], title_before);
        assert!(
            sess.turns.iter().all(|t| t.role != "tool"),
            "Host empty-tools path must not append tool turns"
        );
        let hits = mock.hits.lock().unwrap();
        assert_eq!(hits.len(), 1);
        assert_host_llm_tools_empty(&hits[0]);
    });
}

#[test]
fn run_loop_clarify_under_limit_returns_none_not_wrote() {
    with_sandbox(|| {
        let master = create_bound_plan("歧义计划");
        let mock = spawn_scripted_llm(vec![assistant_text(
            "请问您要改的是主标题，还是某个子计划的标题？",
        )]);
        let mut sess = session::create_session(Some(&master), Some("歧义计划")).unwrap();
        arm_plan_binding(&master);
        let out = r#loop::run_loop(&mut sess, "改标题", &cfg_for(&mock));
        assert_outcome(&out, "none", false);
        assert!(out.reply_text.contains('？') || out.reply_text.contains('?'));
        assert_eq!(r#loop::clarify_count(&sess), 1);
    });
}

#[test]
fn parallel_tool_calls_are_rejected_without_process_dispatch() {
    // Narrowed (P3): Host does not serially dispatch parallel tool_calls.
    with_sandbox(|| {
        let master = create_bound_plan("批处理");
        let before_subs = todo_task::get_by_id(&master)["sub_tasks"]
            .as_array()
            .unwrap()
            .len();
        let mock = spawn_scripted_llm(vec![assistant_tools(
            json!([
                {
                    "id": "c1",
                    "type": "function",
                    "function": {
                        "name": "update_sub_title",
                        "arguments": "{\"sub_task_id\":\"nope\",\"title\":\"x\"}"
                    }
                },
                {
                    "id": "c2",
                    "type": "function",
                    "function": {
                        "name": "add_sub_task",
                        "arguments": "{\"title\":\"批后子项\"}"
                    }
                }
            ]),
            None,
        )]);
        let mut sess = session::create_session(Some(&master), Some("批处理")).unwrap();
        arm_plan_binding(&master);
        let out = r#loop::run_loop(&mut sess, "改不存在的子项并加一个", &cfg_for(&mock));
        assert_outcome(&out, "error", false);
        assert_eq!(
            todo_task::get_by_id(&master)["sub_tasks"]
                .as_array()
                .unwrap()
                .len(),
            before_subs,
            "must not add sub via process-local dispatch"
        );
        assert!(sess.turns.iter().all(|t| t.role != "tool"));
        assert_eq!(mock.hits.lock().unwrap().len(), 1);
        assert_host_llm_tools_empty(&mock.hits.lock().unwrap()[0]);
    });
}

#[test]
fn same_message_tool_calls_plus_content_does_not_dispatch_or_finalize() {
    // Narrowed (P3): tool_calls + content must not drive process-local writes.
    with_sandbox(|| {
        let master = create_bound_plan("同条");
        let before_subs = todo_task::get_by_id(&master)["sub_tasks"]
            .as_array()
            .unwrap()
            .len();
        let mock = spawn_scripted_llm(vec![assistant_tools(
            json!([{
                "id": "c1",
                "type": "function",
                "function": {
                    "name": "add_sub_task",
                    "arguments": "{\"title\":\"同条子项\"}"
                }
            }]),
            Some("这段 content 不是终态"),
        )]);
        let mut sess = session::create_session(Some(&master), Some("同条")).unwrap();
        arm_plan_binding(&master);
        let out = r#loop::run_loop(&mut sess, "加子项", &cfg_for(&mock));
        assert_outcome(&out, "error", false);
        assert_ne!(out.reply_text, "已新增同条子项");
        assert_eq!(
            todo_task::get_by_id(&master)["sub_tasks"]
                .as_array()
                .unwrap()
                .len(),
            before_subs
        );
        assert_host_llm_tools_empty(&mock.hits.lock().unwrap()[0]);
    });
}

#[test]
fn unbound_session_maps_to_business_no_plan_without_llm() {
    with_sandbox(|| {
        let mock = spawn_scripted_llm(vec![assistant_text("should-not-run")]);
        let mut sess = session::create_session(None, None).unwrap();
        let out = r#loop::run_loop(&mut sess, "加个子计划", &cfg_for(&mock));
        assert_outcome(&out, "business", false);
        assert!(
            out.reply_text.contains("Unbound") || out.reply_text.contains("Binding Contract"),
            "{}",
            out.reply_text
        );
        assert_eq!(mock.hits.lock().unwrap().len(), 0);
    });
}

#[test]
fn unknown_tool_name_is_error_terminal_and_does_not_write() {
    with_sandbox(|| {
        let master = create_bound_plan("未知工具");
        let before = todo_task::get_by_id(&master)["title"].clone();
        let mock = spawn_scripted_llm(vec![assistant_tools(
            json!([{
                "id": "c1",
                "type": "function",
                "function": {
                    "name": "delete_master",
                    "arguments": "{}"
                }
            }]),
            None,
        )]);
        let mut sess = session::create_session(Some(&master), Some("未知工具")).unwrap();
        arm_plan_binding(&master);
        let out = r#loop::run_loop(&mut sess, "删掉计划", &cfg_for(&mock));
        assert_outcome(&out, "error", false);
        assert_eq!(todo_task::get_by_id(&master)["title"], before);
        assert!(sess.turns.iter().all(|t| t.role != "tool"));
        assert_host_llm_tools_empty(&mock.hits.lock().unwrap()[0]);
    });
}

#[test]
fn unsupported_tool_calls_upstream_is_error_terminal_no_prompt_json() {
    with_sandbox(|| {
        let master = create_bound_plan("上游");
        let mock = spawn_scripted_llm(vec![(
            400,
            json!({
                "error": {
                    "message": "tools / tool_calls is not supported",
                    "type": "invalid_request_error"
                }
            }),
        )]);
        let mut sess = session::create_session(Some(&master), Some("上游")).unwrap();
        arm_plan_binding(&master);
        let out = r#loop::run_loop(&mut sess, "加子项", &cfg_for(&mock));
        assert_outcome(&out, "error", false);
        let hits = mock.hits.lock().unwrap();
        assert_eq!(hits.len(), 1);
        let blob = hits[0].to_string().to_ascii_lowercase();
        assert!(!blob.contains("respond with json"));
        assert!(!blob.contains("```json"));
    });
}

#[test]
fn length_and_http_errors_map_to_error_terminal_no_retry() {
    with_sandbox(|| {
        let master = create_bound_plan("错误分型");
        for resp in [
            (
                200u16,
                json!({
                    "choices": [{
                        "finish_reason": "length",
                        "message": { "role": "assistant", "content": "cut" }
                    }]
                }),
            ),
            (401, json!({"error":{"message":"no"}})),
            (403, json!({"error":{"message":"forbid"}})),
            (400, json!({"error":{"message":"bad"}})),
            (429, json!({"error":{"message":"slow"}})),
            (500, json!({"error":{"message":"boom"}})),
        ] {
            r#loop::reset_runtime_for_tests();
            let mock = spawn_scripted_llm(vec![resp]);
            let mut sess = session::create_session(Some(&master), Some("错误分型")).unwrap();
        arm_plan_binding(&master);
            let out = r#loop::run_loop(&mut sess, "ping", &cfg_for(&mock));
            assert_outcome(&out, "error", false);
            assert_eq!(mock.hits.lock().unwrap().len(), 1, "no retry");
        }
    });
}

#[test]
fn run_loop_host_text_paths_remain_observable_without_tool_writes() {
    // Narrowed (P3): Host facade chat remains usable via text replies; no tool writes.
    with_sandbox(|| {
        let master = create_bound_plan("子路径");
        let before = todo_task::get_by_id(&master);
        let before_sub_title = before["sub_tasks"][0]["title"].clone();
        let before_len = before["sub_tasks"].as_array().unwrap().len();
        let mock = spawn_scripted_llm(vec![
            assistant_text("已记录新增子计划的请求（Host 本阶段不经 tool_calls 写入）"),
            assistant_text("已记录改子标题的请求（Host 本阶段不经 tool_calls 写入）"),
        ]);
        let mut sess = session::create_session(Some(&master), Some("子路径")).unwrap();
        arm_plan_binding(&master);

        let out_add = r#loop::run_loop(&mut sess, "加一个子计划叫新观察子项", &cfg_for(&mock));
        assert_outcome(&out_add, "none", false);
        assert_eq!(
            todo_task::get_by_id(&master)["sub_tasks"]
                .as_array()
                .unwrap()
                .len(),
            before_len
        );

        let out_upd = r#loop::run_loop(&mut sess, "把原子项标题改成改后子标题", &cfg_for(&mock));
        assert_outcome(&out_upd, "none", false);
        assert_eq!(
            todo_task::get_by_id(&master)["sub_tasks"][0]["title"],
            before_sub_title
        );
        let hits = mock.hits.lock().unwrap();
        assert_eq!(hits.len(), 2);
        assert_host_llm_tools_empty(&hits[0]);
        assert_host_llm_tools_empty(&hits[1]);
    });
}

#[test]
fn terminal_no_plan_unsupported_and_error_are_distinguishable() {
    with_sandbox(|| {
        let mock_unused = spawn_scripted_llm(vec![assistant_text("should-not-run")]);
        let mut unbound = session::create_session(None, None).unwrap();
        let no_plan = r#loop::run_loop(&mut unbound, "加子计划", &cfg_for(&mock_unused));
        assert_outcome(&no_plan, "business", false);
        assert!(
            no_plan.reply_text.contains("Unbound")
                || no_plan.reply_text.contains("Binding Contract"),
            "{}",
            no_plan.reply_text
        );

        let master = create_bound_plan("分型");
        let mock_unsup = spawn_scripted_llm(vec![assistant_text(
            "目前不支持删除计划，我只能查看、加子计划或改标题。",
        )]);
        let mut sess = session::create_session(Some(&master), Some("分型")).unwrap();
        arm_plan_binding(&master);
        let unsupported = r#loop::run_loop(&mut sess, "删掉这个计划", &cfg_for(&mock_unsup));
        assert_outcome(&unsupported, "business", false);
        assert!(
            unsupported.reply_text.contains("目前不支持"),
            "{}",
            unsupported.reply_text
        );

        r#loop::reset_runtime_for_tests();
        let mock_err = spawn_scripted_llm(vec![(
            200,
            json!({
                "choices": [{
                    "finish_reason": "length",
                    "message": { "role": "assistant", "content": "cut" }
                }]
            }),
        )]);
        let mut sess2 = session::create_session(Some(&master), Some("分型")).unwrap();
        arm_plan_binding(&master);
        let tech_err = r#loop::run_loop(&mut sess2, "继续", &cfg_for(&mock_err));
        assert_outcome(&tech_err, "error", false);
        assert!(
            tech_err.reply_text.contains("截断") || tech_err.reply_text.contains("重试"),
            "{}",
            tech_err.reply_text
        );

        assert_ne!(no_plan.reply_text, unsupported.reply_text);
        assert_ne!(no_plan.reply_text, tech_err.reply_text);
        assert_ne!(unsupported.reply_text, tech_err.reply_text);
        assert_ne!(
            format!("{:?}", no_plan.terminal),
            format!("{:?}", tech_err.terminal)
        );
        assert_eq!(unsupported.terminal, Terminal::Business);
        assert_eq!(tech_err.terminal, Terminal::Error);
    });
}

#[test]
fn malformed_response_is_error_and_does_not_write() {
    with_sandbox(|| {
        let master = create_bound_plan("畸形");
        let before_subs = todo_task::get_by_id(&master)["sub_tasks"]
            .as_array()
            .unwrap()
            .len();
        let mock = spawn_scripted_llm(vec![(200, json!({}))]);
        let mut sess = session::create_session(Some(&master), Some("畸形")).unwrap();
        arm_plan_binding(&master);
        let out = r#loop::run_loop(&mut sess, "加子项", &cfg_for(&mock));
        assert_outcome(&out, "error", false);
        assert_eq!(
            todo_task::get_by_id(&master)["sub_tasks"]
                .as_array()
                .unwrap()
                .len(),
            before_subs
        );
    });
}

#[test]
fn missing_tool_call_id_is_error_no_write() {
    with_sandbox(|| {
        let master = create_bound_plan("缺id");
        let mock = spawn_scripted_llm(vec![assistant_tools(
            json!([{
                "id": "",
                "type": "function",
                "function": {
                    "name": "add_sub_task",
                    "arguments": "{\"title\":\"x\"}"
                }
            }]),
            None,
        )]);
        let mut sess = session::create_session(Some(&master), Some("缺id")).unwrap();
        arm_plan_binding(&master);
        let out = r#loop::run_loop(&mut sess, "加", &cfg_for(&mock));
        assert_outcome(&out, "error", false);
    });
}

#[test]
fn tool_rounds_hard_cap_eight_errors() {
    // Narrowed (P3): first unexpected tool_calls stops the turn; no multi-round dispatch.
    with_sandbox(|| {
        let master = create_bound_plan("工具上限");
        let mock = spawn_scripted_llm(vec![assistant_tools(
            json!([{
                "id": "c0",
                "type": "function",
                "function": {
                    "name": "get_plan",
                    "arguments": "{}"
                }
            }]),
            None,
        )]);
        let mut sess = session::create_session(Some(&master), Some("工具上限")).unwrap();
        arm_plan_binding(&master);
        let out = r#loop::run_loop(&mut sess, "一直读", &cfg_for(&mock));
        assert_outcome(&out, "error", false);
        let tool_rounds = sess.turns.iter().filter(|t| t.role == "tool").count();
        assert_eq!(tool_rounds, 0, "Host empty-tools must not enter tool rounds");
        assert_eq!(mock.hits.lock().unwrap().len(), 1);
        assert_host_llm_tools_empty(&mock.hits.lock().unwrap()[0]);
    });
}

#[test]
fn clarify_rounds_hard_cap_five_errors() {
    with_sandbox(|| {
        let master = create_bound_plan("澄清上限");
        let mut sess = session::create_session(Some(&master), Some("澄清上限")).unwrap();
        arm_plan_binding(&master);
        for i in 0..5 {
            let mock = spawn_scripted_llm(vec![assistant_text(&format!(
                "还需要补充信息吗？（第{}次）？",
                i + 1
            ))]);
            let out = r#loop::run_loop(&mut sess, &format!("改标题{i}"), &cfg_for(&mock));
            assert_eq!(out.terminal, Terminal::None);
        }
        let mock = spawn_scripted_llm(vec![assistant_text("再问一次？")]);
        let out = r#loop::run_loop(&mut sess, "还是改标题", &cfg_for(&mock));
        assert_outcome(&out, "error", false);
    });
}

#[test]
fn open_ai_assistant_idle_creates_session_and_returns_binding() {
    with_sandbox(|| {
        let master = create_bound_plan("打开助手");
        let v = r#loop::open_ai_assistant_core(&master).expect("open");
        assert!(v["session_id"].as_str().unwrap().starts_with("sess_"));
        assert!(v.get("bound_master_task_id").is_none());
        assert!(v["session_id"].as_str().unwrap_or("").starts_with("sess_"));
        assert_eq!(v["window_label"], "ai-assistant");
        assert_ne!(v["busy"], true);
    });
}

#[test]
fn open_ai_assistant_busy_rejects_rebind() {
    with_sandbox(|| {
        let a = create_bound_plan("计划A");
        let b = create_bound_plan("计划B");
        arm_plan_binding(&a);
        let first = r#loop::open_ai_assistant_core(&a).unwrap();
        let sid = first["session_id"].as_str().unwrap().to_string();
        r#loop::set_busy_for_tests(true);
        // Replace Set while busy still cuts the live session (clear-first).
        arm_plan_binding(&b);
        assert_eq!(
            live_session_id(),
            None,
            "replace Set clears live session even while busy"
        );
        let second = r#loop::open_ai_assistant_core(&b).unwrap();
        assert_eq!(second["busy"], true);
        // Busy open must not mint a new session; cut leaves no live id to echo.
        assert!(
            second["session_id"].is_null()
                || second["session_id"].as_str().unwrap_or("").is_empty(),
            "busy open after session cut must not invent a live session; got {}",
            second["session_id"]
        );
        assert_ne!(
            second["session_id"].as_str().unwrap_or(""),
            sid,
            "cut old session must not remain the busy-open live id"
        );
        assert!(second.get("bound_master_task_id").is_none());
        assert!(second.get("bound_title").is_none());
    });
}

#[test]
fn agent_chat_turn_happy_path_emits_turn_completed() {
    with_sandbox(|| {
        let master = create_bound_plan("对话");
        let mock = spawn_scripted_llm(vec![assistant_text("收到")]);
        install_llm_cfg(&mock);
        arm_plan_binding(&master);
        let open = r#loop::open_ai_assistant_core(&master).unwrap();
        let sid = open["session_id"].as_str().unwrap();
        let result = r#loop::agent_chat_turn_core(sid, "你好", Some(&master)).unwrap();
        assert_eq!(result.body["terminal"], "none");
        assert_eq!(result.body["wrote"], false);
        assert_eq!(result.body["busy"], false);
        assert!(result.body.get("bound_master_task_id").is_none());
        assert!(result.body["reply_text"].as_str().unwrap().contains("收到"));
        let ev = result.emit_turn_completed.expect("must emit");
        assert_eq!(ev["event"], EVENT_TURN_COMPLETED);
        assert_eq!(ev["payload"]["session_id"], sid);
        assert_eq!(ev["payload"]["wrote"], false);
        assert_eq!(ev["payload"]["terminal"], "none");
        assert!(ev["payload"].get("bound_master_task_id").is_none());
    });
}

#[test]
fn agent_chat_turn_busy_rejects_without_emit_or_user_turn() {
    with_sandbox(|| {
        let master = create_bound_plan("忙");
        arm_plan_binding(&master);
        let open = r#loop::open_ai_assistant_core(&master).unwrap();
        let sid = open["session_id"].as_str().unwrap().to_string();
        let before = session::load_session(&sid).unwrap();
        let before_len = before.turns.len();
        r#loop::set_busy_for_tests(true);
        let result = r#loop::agent_chat_turn_core(&sid, "第二路", Some(&master)).unwrap();
        assert_eq!(result.body["busy"], true);
        assert!(result.emit_turn_completed.is_none());
        let after = session::load_session(&sid).unwrap();
        assert_eq!(after.turns.len(), before_len, "user must not enter session");
    });
}

#[test]
fn agent_chat_turn_ignores_master_arg_uses_binding_ctx() {
    with_sandbox(|| {
        let a = create_bound_plan("绑定A");
        let b = create_bound_plan("其它B");
        let mock = spawn_scripted_llm(vec![assistant_text("ok")]);
        install_llm_cfg(&mock);
        arm_plan_binding(&a);
        let open = r#loop::open_ai_assistant_core(&a).unwrap();
        let sid = open["session_id"].as_str().unwrap();
        // Client master arg is ignored; Binding.tools ctx drives execution.
        let result = r#loop::agent_chat_turn_core(sid, "你好", Some(&b)).unwrap();
        assert_eq!(result.body["terminal"], "none");
        assert_eq!(result.body["wrote"], false);
        assert!(result.emit_turn_completed.is_some());
        assert_eq!(mock.hits.lock().unwrap().len(), 1);
        assert!(result.body.get("bound_master_task_id").is_none());
    });
}

#[test]
fn in_flight_chat_keeps_binding_despite_busy_open_without_tool_writes() {
    // Narrowed (P3): busy open still rejected; Host chat is text-only (wrote=false).
    with_sandbox(|| {
        let a = create_bound_plan("旧绑定");
        let b = create_bound_plan("新绑定");
        let title_a = todo_task::get_by_id(&a)["title"].clone();
        let mock = spawn_scripted_llm(vec![assistant_text("已理解改标题请求（无进程内写入）")]);
        install_llm_cfg(&mock);
        arm_plan_binding(&a);
        let open = r#loop::open_ai_assistant_core(&a).unwrap();
        let sid = open["session_id"].as_str().unwrap().to_string();

        r#loop::set_busy_for_tests(true);
        // Busy open must not swap Binding.
        let rejected = r#loop::open_ai_assistant_core(&b).unwrap();
        assert_eq!(rejected["busy"], true);
        assert!(rejected.get("bound_master_task_id").is_none());
        r#loop::set_busy_for_tests(false);

        let result = r#loop::agent_chat_turn_core(&sid, "改标题", Some(&a)).unwrap();
        assert_eq!(result.body["wrote"], false);
        assert_eq!(result.body["terminal"], "none");
        assert_eq!(todo_task::get_by_id(&a)["title"], title_a);
        assert_eq!(todo_task::get_by_id(&b)["title"], "新绑定");
        assert_host_llm_tools_empty(&mock.hits.lock().unwrap()[0]);
    });
}

#[test]
fn close_window_does_not_abort_emit_still_available() {
    with_sandbox(|| {
        let master = create_bound_plan("关窗");
        let mock = spawn_scripted_llm(vec![assistant_text("跑完了")]);
        install_llm_cfg(&mock);
        arm_plan_binding(&master);
        let open = r#loop::open_ai_assistant_core(&master).unwrap();
        let sid = open["session_id"].as_str().unwrap();
        let result = r#loop::agent_chat_turn_core(sid, "继续", Some(&master)).unwrap();
        assert!(result.emit_turn_completed.is_some());
        assert_eq!(result.body["reply_text"], "跑完了");
    });
}

#[test]
fn event_name_is_literally_ai_assistant_turn_completed() {
    assert_eq!(EVENT_TURN_COMPLETED, "ai-assistant:turn-completed");
}

// --- Binding Contract B1: tools + prompt + callbacks Set validation (t1) ---

fn applicable_tools() -> Value {
    json!([{ "name": "tool_a", "handle": "opaque-tool-a" }])
}

fn applicable_prompt() -> Value {
    json!("opaque-system-prompt")
}

fn empty_callbacks_registry() -> Value {
    json!({})
}

fn valid_binding() -> r#loop::Binding {
    r#loop::Binding {
        tools: applicable_tools(),
        prompt: applicable_prompt(),
        callbacks: empty_callbacks_registry(),
    }
}

#[test]
fn set_binding_accepts_valid_tools_prompt_and_callbacks() {
    with_sandbox(|| {
        assert_eq!(r#loop::binding_state(), "unbound");
        let result = r#loop::set_binding(valid_binding());
        assert!(
            result.is_ok(),
            "valid Binding must pass Set validation, got {result:?}"
        );
        assert_ne!(
            result.err().map(|e| e.as_code()),
            Some("set_invalid")
        );
        assert_eq!(
            r#loop::binding_state(),
            "bound",
            "valid Set must enter subsequent bound path"
        );
    });
}

#[test]
fn binding_config_surface_is_generic_tools_prompt_callbacks() {
    with_sandbox(|| {
        let binding = valid_binding();
        let encoded = serde_json::to_value(&binding).expect("encode Binding");
        let obj = encoded.as_object().expect("Binding encodes as object");
        assert!(obj.contains_key("tools"), "tools slot required");
        assert!(obj.contains_key("prompt"), "prompt slot required");
        assert!(obj.contains_key("callbacks"), "callbacks slot required");
        assert!(
            !obj.contains_key("master_task_id"),
            "business field master_task_id must not be part of Binding contract surface"
        );
        assert!(
            !obj.contains_key("bound_master_task_id"),
            "business field bound_master_task_id must not be part of Binding contract surface"
        );
        r#loop::set_binding(binding).expect("generic Binding must Set successfully");
        assert_eq!(r#loop::binding_state(), "bound");
    });
}

#[test]
fn set_binding_allows_empty_callbacks_registry_when_slot_present() {
    with_sandbox(|| {
        let binding = r#loop::Binding {
            tools: applicable_tools(),
            prompt: applicable_prompt(),
            callbacks: json!({}),
        };
        assert!(r#loop::set_binding(binding).is_ok());
        assert_eq!(r#loop::binding_state(), "bound");
    });
}

#[test]
fn set_binding_allows_empty_tools_array_but_rejects_empty_prompt() {
    with_sandbox(|| {
        // L1+L2: empty tools array is legal (Host Agent business session tools empty).
        let empty_tools = r#loop::Binding {
            tools: json!([]),
            prompt: applicable_prompt(),
            callbacks: empty_callbacks_registry(),
        };
        r#loop::set_binding(empty_tools).expect("empty tools array must Set");
        assert_eq!(r#loop::binding_state(), "bound");
        r#loop::reset_binding().expect("reset");

        let empty_prompt = r#loop::Binding {
            tools: applicable_tools(),
            prompt: json!(""),
            callbacks: empty_callbacks_registry(),
        };
        let err = r#loop::set_binding(empty_prompt).expect_err("empty prompt must fail");
        assert_eq!(err.as_code(), "set_invalid");
        assert_eq!(r#loop::binding_state(), "unbound");
    });
}

#[test]
fn set_binding_rejects_missing_slot_keeps_prior_state() {
    with_sandbox(|| {
        // Missing any of tools / prompt / callbacks → set_invalid, stay unbound.
        for payload in [
            json!({ "prompt": "p", "callbacks": {} }),
            json!({ "tools": [{ "name": "t" }], "callbacks": {} }),
            json!({ "tools": [{ "name": "t" }], "prompt": "p" }),
        ] {
            let err = r#loop::try_set_binding_json(&payload)
                .expect_err("missing slot must fail Set");
            assert_eq!(err.as_code(), "set_invalid");
            assert_eq!(r#loop::binding_state(), "unbound");
        }

        // After a successful Set, illegal Set must not rewrite bound state.
        r#loop::set_binding(valid_binding()).expect("seed bound");
        assert_eq!(r#loop::binding_state(), "bound");
        let err = r#loop::try_set_binding_json(&json!({
            "tools": [{ "name": "t" }],
            "prompt": "p"
            // callbacks slot absent
        }))
        .expect_err("missing callbacks must fail");
        assert_eq!(err.as_code(), "set_invalid");
        assert_eq!(
            r#loop::binding_state(),
            "bound",
            "illegal Set must keep prior bound state"
        );
    });
}

#[test]
fn set_binding_does_not_assemble_or_default_fill_from_business_fields() {
    with_sandbox(|| {
        // Business-only payload without tools/prompt must not be auto-completed into success.
        let err = r#loop::try_set_binding_json(&json!({
            "master_task_id": "task_business_only",
            "callbacks": {}
        }))
        .expect_err("Host must not fill tools/prompt from business fields");
        assert_eq!(err.as_code(), "set_invalid");
        assert_eq!(r#loop::binding_state(), "unbound");

        let err = r#loop::try_set_binding_json(&json!({
            "bound_master_task_id": "task_x",
            "tools": [],
            "prompt": "",
            "callbacks": {}
        }))
        .expect_err("empty tools/prompt must not succeed via business-field side path");
        assert_eq!(err.as_code(), "set_invalid");
        assert_eq!(r#loop::binding_state(), "unbound");
    });
}

// --- Binding Contract state machine: Set / Reset / query + serialization (t2) ---

fn binding_with_prompt(prompt: &str) -> r#loop::Binding {
    r#loop::Binding {
        tools: applicable_tools(),
        prompt: json!(prompt),
        callbacks: empty_callbacks_registry(),
    }
}

fn assert_query_unbound(summary: &r#loop::BindingStateSummary) {
    assert_eq!(summary.state, "unbound");
    assert!(
        summary.generation.is_none(),
        "unbound query must not retain a live binding generation: {summary:?}"
    );
}

fn assert_query_bound(summary: &r#loop::BindingStateSummary) {
    assert_eq!(summary.state, "bound");
    assert!(
        summary.generation.is_some(),
        "bound query must expose a generation for discard/replace proofs: {summary:?}"
    );
}

fn assert_query_is_business_agnostic(summary: &r#loop::BindingStateSummary) {
    let encoded = serde_json::to_value(summary).expect("encode BindingStateSummary");
    let obj = encoded
        .as_object()
        .expect("BindingStateSummary encodes as object");
    assert!(
        !obj.contains_key("tools"),
        "query must not return tools slot: {encoded}"
    );
    assert!(
        !obj.contains_key("prompt"),
        "query must not return prompt slot: {encoded}"
    );
    assert!(
        !obj.contains_key("callbacks"),
        "query must not return callbacks slot: {encoded}"
    );
    assert!(
        !obj.contains_key("master_task_id"),
        "query must not return business id master_task_id: {encoded}"
    );
    assert!(
        !obj.contains_key("bound_master_task_id"),
        "query must not return business id bound_master_task_id: {encoded}"
    );
    let blob = encoded.to_string();
    assert!(
        !blob.contains("opaque-system-prompt")
            && !blob.contains("opaque-tool-a")
            && !blob.contains("secret-prompt-body")
            && !blob.contains("task_business"),
        "query summary must not leak tools/prompt body or business ids: {blob}"
    );
}

#[test]
fn set_then_query_returns_bound_business_agnostic_summary() {
    with_sandbox(|| {
        assert_query_unbound(&r#loop::query_binding());
        r#loop::set_binding(binding_with_prompt("secret-prompt-body")).expect("Set");
        let q = r#loop::query_binding();
        assert_query_bound(&q);
        assert_query_is_business_agnostic(&q);
        assert_eq!(r#loop::binding_state(), "bound");
        assert_eq!(
            r#loop::current_binding_slot_count(),
            1,
            "after Set there must be exactly one current Binding"
        );
    });
}

#[test]
fn reset_bound_to_unbound_discards_current_binding() {
    with_sandbox(|| {
        r#loop::set_binding(valid_binding()).expect("Set");
        let before = r#loop::query_binding();
        assert_query_bound(&before);
        let gen = before.generation.expect("bound gen");

        r#loop::reset_binding().expect("Reset");
        let after = r#loop::query_binding();
        assert_query_unbound(&after);
        assert_eq!(r#loop::binding_state(), "unbound");
        assert_eq!(
            r#loop::current_binding_slot_count(),
            0,
            "Reset must discard the current Binding reference"
        );
        assert!(
            !r#loop::is_binding_generation_current(gen),
            "old Binding generation must be discarded after Reset"
        );
    });
}

#[test]
fn set_on_bound_atomically_replaces_and_invalidates_old_binding() {
    with_sandbox(|| {
        r#loop::set_binding(binding_with_prompt("prompt-v1")).expect("first Set");
        let first = r#loop::query_binding();
        assert_query_bound(&first);
        let old_gen = first.generation.expect("old gen");

        r#loop::set_binding(binding_with_prompt("prompt-v2")).expect("replace Set");
        let second = r#loop::query_binding();
        assert_query_bound(&second);
        assert_query_is_business_agnostic(&second);
        let new_gen = second.generation.expect("new gen");
        assert_ne!(old_gen, new_gen, "replace Set must mint a new binding generation");
        assert!(
            !r#loop::is_binding_generation_current(old_gen),
            "old Binding must be immediately invalid after replace Set"
        );
        assert!(r#loop::is_binding_generation_current(new_gen));
        assert_eq!(
            r#loop::current_binding_slot_count(),
            1,
            "replace must leave exactly one current Binding"
        );
        assert_eq!(r#loop::binding_state(), "bound");
    });
}

#[test]
fn reset_already_unbound_is_idempotent_success() {
    with_sandbox(|| {
        assert_query_unbound(&r#loop::query_binding());
        r#loop::reset_binding().expect("Reset unbound must succeed");
        assert_query_unbound(&r#loop::query_binding());
        assert_eq!(r#loop::binding_state(), "unbound");
        assert_eq!(r#loop::current_binding_slot_count(), 0);

        r#loop::reset_binding().expect("second Reset unbound must succeed");
        assert_query_unbound(&r#loop::query_binding());
    });
}

#[test]
fn host_runtime_reset_starts_unbound_before_successful_set() {
    with_sandbox(|| {
        // with_sandbox already calls reset_runtime_for_tests (Host runtime reset).
        let q = r#loop::query_binding();
        assert_query_unbound(&q);
        assert_eq!(r#loop::binding_state(), "unbound");
        assert_eq!(r#loop::current_binding_slot_count(), 0);

        r#loop::reset_runtime_for_tests();
        assert_query_unbound(&r#loop::query_binding());
        assert_eq!(r#loop::binding_state(), "unbound");
    });
}

#[test]
fn query_does_not_change_binding_state() {
    with_sandbox(|| {
        assert_query_unbound(&r#loop::query_binding());
        let _ = r#loop::query_binding();
        assert_query_unbound(&r#loop::query_binding());
        assert_eq!(r#loop::current_binding_slot_count(), 0);

        r#loop::set_binding(valid_binding()).expect("Set");
        let before = r#loop::query_binding();
        assert_query_bound(&before);
        let gen = before.generation;
        let slot = r#loop::current_binding_slot_count();

        let mid = r#loop::query_binding();
        assert_eq!(mid.state, before.state);
        assert_eq!(mid.generation, gen);
        assert_eq!(r#loop::current_binding_slot_count(), slot);
        assert_eq!(r#loop::binding_state(), "bound");
    });
}

#[test]
fn illegal_set_does_not_transition_to_bound() {
    with_sandbox(|| {
        assert_eq!(r#loop::binding_state(), "unbound");
        // Legacy/null tools payload remains illegal; empty array is legal elsewhere (L1+L2).
        let err = r#loop::try_set_binding_json(&json!({
            "tools": null,
            "prompt": "p",
            "callbacks": {}
        }))
        .expect_err("illegal Set must fail");
        assert_eq!(err.as_code(), "set_invalid");
        assert_query_unbound(&r#loop::query_binding());
        assert_eq!(r#loop::current_binding_slot_count(), 0);

        r#loop::set_binding(valid_binding()).expect("seed bound");
        let before = r#loop::query_binding();
        assert_query_bound(&before);
        let gen = before.generation;

        let err = r#loop::try_set_binding_json(&json!({
            "tools": [{ "name": "t" }],
            "prompt": "p"
            // callbacks missing
        }))
        .expect_err("illegal Set must fail");
        assert_eq!(err.as_code(), "set_invalid");
        let after = r#loop::query_binding();
        assert_query_bound(&after);
        assert_eq!(after.generation, gen, "illegal Set must not rewrite current Binding");
        assert_eq!(r#loop::current_binding_slot_count(), 1);
    });
}

#[test]
fn host_serializes_set_reset_at_most_one_current_binding() {
    with_sandbox(|| {
        let barrier = Arc::new(std::sync::Barrier::new(8));
        let max_slots = Arc::new(Mutex::new(0usize));
        let errors = Arc::new(Mutex::new(Vec::new()));

        let mut handles = Vec::new();
        for i in 0..8 {
            let barrier = barrier.clone();
            let max_slots = max_slots.clone();
            let errors = errors.clone();
            handles.push(thread::spawn(move || {
                barrier.wait();
                for round in 0..40 {
                    let binding = binding_with_prompt(&format!("prompt-{i}-{round}"));
                    if let Err(e) = r#loop::set_binding(binding) {
                        errors
                            .lock()
                            .unwrap()
                            .push(format!("set failed: {}", e.as_code()));
                    }
                    let (q_after_set, slots) = r#loop::query_binding_snapshot();
                    {
                        let mut m = max_slots.lock().unwrap();
                        if slots > *m {
                            *m = slots;
                        }
                    }
                    if slots > 1 {
                        errors
                            .lock()
                            .unwrap()
                            .push(format!("observed {slots} current bindings"));
                    }
                    if q_after_set.state == "bound" && slots != 1 {
                        errors.lock().unwrap().push(format!(
                            "query bound but slot_count={slots}"
                        ));
                    }
                    if q_after_set.state == "unbound" && slots != 0 {
                        errors.lock().unwrap().push(format!(
                            "query unbound but slot_count={slots}"
                        ));
                    }
                    if round % 2 == 0 {
                        if let Err(_) = r#loop::reset_binding() {
                            errors.lock().unwrap().push("reset failed".into());
                        }
                    }
                    let (q, slots_after) = r#loop::query_binding_snapshot();
                    match q.state {
                        "unbound" => {
                            if slots_after != 0 {
                                errors.lock().unwrap().push(format!(
                                    "query unbound but slot_count={slots_after}"
                                ));
                            }
                        }
                        "bound" => {
                            if slots_after != 1 {
                                errors.lock().unwrap().push(format!(
                                    "query bound but slot_count={slots_after}"
                                ));
                            }
                        }
                        other => errors
                            .lock()
                            .unwrap()
                            .push(format!("unexpected state {other}")),
                    }
                }
            }));
        }

        for h in handles {
            h.join().expect("thread join");
        }
        let errs = errors.lock().unwrap().clone();
        assert!(
            errs.is_empty(),
            "serialization / dual-binding violations: {errs:?}"
        );
        assert!(
            *max_slots.lock().unwrap() <= 1,
            "must never observe more than one current Binding"
        );
        // Final state is well-formed: 0 or 1 slot matching query.
        let (final_q, final_slots) = r#loop::query_binding_snapshot();
        match final_q.state {
            "unbound" => assert_eq!(final_slots, 0),
            "bound" => assert_eq!(final_slots, 1),
            other => panic!("unexpected final state {other}"),
        }
    });
}

// --- execute gate + mid-execute Reset (strategy A) + lifecycle callbacks (t3) ---

fn event_names(events: &[r#loop::LifecycleEvent]) -> Vec<&str> {
    events.iter().map(|e| e.event).collect()
}

fn assert_payload_contract_only(ev: &r#loop::LifecycleEvent) {
    let encoded = serde_json::to_value(ev).expect("encode LifecycleEvent");
    let obj = encoded
        .as_object()
        .expect("LifecycleEvent encodes as object");
    assert!(obj.contains_key("event"), "event name required: {encoded}");
    // category may be absent for onBound/onUnbound
    for forbidden in [
        "tools",
        "prompt",
        "callbacks",
        "master_task_id",
        "bound_master_task_id",
        "binding",
        "generation",
        "applied_tools",
        "applied_prompt",
    ] {
        assert!(
            !obj.contains_key(forbidden),
            "callback payload must not carry {forbidden}: {encoded}"
        );
    }
    let blob = encoded.to_string();
    assert!(
        !blob.contains("opaque-system-prompt")
            && !blob.contains("opaque-tool-a")
            && !blob.contains("secret-prompt")
            && !blob.contains("task_business"),
        "callback payload must not leak tools/prompt body or business ids: {blob}"
    );
}

#[test]
fn execute_when_bound_uses_current_binding_tools_and_prompt() {
    with_sandbox(|| {
        r#loop::set_binding(binding_with_prompt("prompt-exec-v1")).expect("Set");
        let out = r#loop::execute_binding().expect("bound execute must succeed");
        assert_eq!(out.applied_prompt, json!("prompt-exec-v1"));
        assert_eq!(out.applied_tools, applicable_tools());
        assert_eq!(r#loop::binding_state(), "bound");
    });
}

#[test]
fn set_success_emits_on_bound_synchronously() {
    with_sandbox(|| {
        r#loop::clear_lifecycle_events_for_tests();
        r#loop::set_binding(valid_binding()).expect("Set");
        let events = r#loop::drain_lifecycle_events();
        assert_eq!(event_names(&events), vec!["onBound"]);
        assert_payload_contract_only(&events[0]);
        assert!(events[0].category.is_none());
    });
}

#[test]
fn reset_to_unbound_emits_on_unbound() {
    with_sandbox(|| {
        r#loop::set_binding(valid_binding()).expect("Set");
        r#loop::clear_lifecycle_events_for_tests();
        r#loop::reset_binding().expect("Reset");
        let events = r#loop::drain_lifecycle_events();
        assert_eq!(event_names(&events), vec!["onUnbound"]);
        assert_payload_contract_only(&events[0]);
        assert_eq!(r#loop::binding_state(), "unbound");
    });
}

#[test]
fn replace_set_emits_on_unbound_then_on_bound_and_execute_uses_new() {
    with_sandbox(|| {
        r#loop::set_binding(binding_with_prompt("prompt-old")).expect("first Set");
        let old_gen = r#loop::query_binding().generation.expect("old gen");
        r#loop::clear_lifecycle_events_for_tests();

        r#loop::set_binding(binding_with_prompt("prompt-new")).expect("replace Set");
        let events = r#loop::drain_lifecycle_events();
        assert_eq!(
            event_names(&events),
            vec!["onUnbound", "onBound"],
            "D1 replace sequence must be onUnbound → onBound"
        );
        for ev in &events {
            assert_payload_contract_only(ev);
        }
        assert!(!r#loop::is_binding_generation_current(old_gen));

        let out = r#loop::execute_binding().expect("execute after replace");
        assert_eq!(out.applied_prompt, json!("prompt-new"));
        assert_ne!(out.applied_prompt, json!("prompt-old"));
    });
}

#[test]
fn lifecycle_callbacks_delivered_synchronously_in_call_path_order() {
    with_sandbox(|| {
        let order = Arc::new(Mutex::new(Vec::new()));
        let order_c = order.clone();
        r#loop::set_lifecycle_listener_for_tests(Some(Box::new(move |ev| {
            order_c.lock().unwrap().push(ev.event.to_string());
        })));

        r#loop::set_binding(binding_with_prompt("a")).expect("Set");
        // Listener saw onBound before set_binding returned (sync).
        assert_eq!(order.lock().unwrap().clone(), vec!["onBound".to_string()]);

        r#loop::set_binding(binding_with_prompt("b")).expect("replace");
        assert_eq!(
            order.lock().unwrap().clone(),
            vec![
                "onBound".to_string(),
                "onUnbound".to_string(),
                "onBound".to_string()
            ]
        );
    });
}

#[test]
fn mid_execute_reset_strategy_a_unbounds_cancels_and_emits() {
    with_sandbox(|| {
        r#loop::set_binding(binding_with_prompt("in-flight")).expect("Set");
        let gen = r#loop::query_binding().generation.expect("gen");
        r#loop::clear_lifecycle_events_for_tests();

        let err = r#loop::execute_binding_during(|| {
            r#loop::reset_binding().expect("Reset during execute");
            assert_eq!(
                r#loop::binding_state(),
                "unbound",
                "strategy A: immediately unbound"
            );
            assert!(!r#loop::is_binding_generation_current(gen));
        })
        .expect_err("in-flight execute must cancel/fail");
        assert_eq!(err.as_code(), "reset_cancelled");
        assert_eq!(r#loop::binding_state(), "unbound");

        let events = r#loop::drain_lifecycle_events();
        assert!(
            events.iter().any(|e| e.event == "onUnbound"),
            "must observe onUnbound: {events:?}"
        );
        let err_ev = events
            .iter()
            .find(|e| e.event == "onError")
            .expect("must observe onError(reset_cancelled)");
        assert_eq!(err_ev.category, Some("reset_cancelled"));
        assert_payload_contract_only(err_ev);

        // Order: unbound first, then cancel error on the execute path.
        let names = event_names(&events);
        let i_unbound = names.iter().position(|n| *n == "onUnbound").unwrap();
        let i_err = names.iter().position(|n| *n == "onError").unwrap();
        assert!(
            i_unbound < i_err,
            "onUnbound before onError(reset_cancelled): {names:?}"
        );
    });
}

#[test]
fn reset_already_unbound_does_not_repeat_on_unbound() {
    with_sandbox(|| {
        assert_eq!(r#loop::binding_state(), "unbound");
        r#loop::clear_lifecycle_events_for_tests();
        r#loop::reset_binding().expect("idempotent Reset");
        r#loop::reset_binding().expect("second idempotent Reset");
        let events = r#loop::drain_lifecycle_events();
        assert!(
            events.iter().all(|e| e.event != "onUnbound"),
            "idempotent Reset must not emit onUnbound: {events:?}"
        );
    });
}

#[test]
fn no_listener_still_advances_state_machine() {
    with_sandbox(|| {
        r#loop::set_lifecycle_listener_for_tests(None);
        r#loop::set_binding(valid_binding()).expect("Set without listener");
        assert_eq!(r#loop::binding_state(), "bound");
        r#loop::reset_binding().expect("Reset without listener");
        assert_eq!(r#loop::binding_state(), "unbound");
        // Events are still recorded (emission ≠ requiring a listener).
        let events = r#loop::drain_lifecycle_events();
        assert!(
            events.iter().any(|e| e.event == "onBound"),
            "events still emitted without listener: {events:?}"
        );
        assert!(
            events.iter().any(|e| e.event == "onUnbound"),
            "events still emitted without listener: {events:?}"
        );
    });
}

#[test]
fn execute_when_unbound_hard_fails_with_rejected_unbound() {
    with_sandbox(|| {
        assert_eq!(r#loop::binding_state(), "unbound");
        r#loop::clear_lifecycle_events_for_tests();
        let err = r#loop::execute_binding().expect_err("unbound execute must hard-fail");
        assert_eq!(err.as_code(), "rejected_unbound");
        let events = r#loop::drain_lifecycle_events();
        let err_ev = events
            .iter()
            .find(|e| e.event == "onError")
            .expect("onError(rejected_unbound)");
        assert_eq!(err_ev.category, Some("rejected_unbound"));
        assert_payload_contract_only(err_ev);
        assert_eq!(r#loop::binding_state(), "unbound");
    });
}

#[test]
fn execute_after_reset_rejects_and_old_config_not_reused() {
    with_sandbox(|| {
        r#loop::set_binding(binding_with_prompt("old-only")).expect("Set");
        let before = r#loop::execute_binding().expect("execute while bound");
        assert_eq!(before.applied_prompt, json!("old-only"));

        r#loop::reset_binding().expect("Reset");
        r#loop::clear_lifecycle_events_for_tests();
        let err = r#loop::execute_binding().expect_err("execute after Reset must reject");
        assert_eq!(err.as_code(), "rejected_unbound");
        let events = r#loop::drain_lifecycle_events();
        assert!(
            events
                .iter()
                .any(|e| e.event == "onError" && e.category == Some("rejected_unbound")),
            "{events:?}"
        );
        // No successful apply of old prompt after Reset.
        assert!(
            events
                .iter()
                .all(|e| e.event != "onBound"),
            "must not re-bind via execute: {events:?}"
        );
    });
}

#[test]
fn present_without_set_leaves_unbound_and_execute_rejects() {
    with_sandbox(|| {
        let master = create_bound_plan("仅Present");
        let _ = r#loop::open_ai_assistant_core(&master).expect("Present/open shell");
        assert_eq!(
            r#loop::binding_state(),
            "unbound",
            "Present/open must not imply Binding Contract bound"
        );
        let err = r#loop::execute_binding().expect_err("execute without Set");
        assert_eq!(err.as_code(), "rejected_unbound");
    });
}

#[test]
fn listener_panic_does_not_rollback_state_and_emits_callback_failed() {
    with_sandbox(|| {
        r#loop::set_lifecycle_listener_for_tests(Some(Box::new(|_| {
            panic!("listener boom");
        })));
        r#loop::clear_lifecycle_events_for_tests();
        r#loop::set_binding(valid_binding()).expect("Set must succeed despite listener panic");
        assert_eq!(
            r#loop::binding_state(),
            "bound",
            "listener failure must not roll back state machine"
        );
        let events = r#loop::drain_lifecycle_events();
        assert!(
            events.iter().any(|e| e.event == "onBound"),
            "onBound still emitted: {events:?}"
        );
        let err_ev = events
            .iter()
            .find(|e| e.event == "onError" && e.category == Some("callback_failed"))
            .expect("onError(callback_failed)");
        assert_payload_contract_only(err_ev);
    });
}

#[test]
fn callback_payload_is_event_plus_category_only() {
    with_sandbox(|| {
        r#loop::clear_lifecycle_events_for_tests();
        r#loop::set_binding(binding_with_prompt("secret-prompt-body")).expect("Set");
        let _ = r#loop::execute_binding().expect("bound ok");
        r#loop::reset_binding().expect("Reset");
        let _ = r#loop::execute_binding().expect_err("rejected");
        for ev in r#loop::drain_lifecycle_events() {
            assert_payload_contract_only(&ev);
        }
    });
}

// --- Present shell surface: Present≠Set≠bound; not Binding Contract ops (t4) ---

#[test]
fn binding_contract_ops_exclude_present_and_open() {
    with_sandbox(|| {
        let ops = r#loop::binding_contract_ops();
        let normalized: Vec<String> = ops
            .iter()
            .map(|s| s.to_ascii_lowercase())
            .collect();
        for required in ["set", "reset", "query", "execute"] {
            assert!(
                normalized.iter().any(|op| op == required),
                "Binding Contract ops must include {required}; got {ops:?}"
            );
        }
        assert!(
            normalized.iter().any(|op| op.contains("callback")),
            "Binding Contract ops must include callbacks surface; got {ops:?}"
        );
        for forbidden in ["present", "open"] {
            assert!(
                normalized
                    .iter()
                    .all(|op| op != forbidden && !op.contains(forbidden)),
                "Present/Open must not appear in Binding Contract ops: {ops:?}"
            );
        }
    });
}

#[test]
fn present_ai_assistant_maps_to_shell_focus_without_set() {
    with_sandbox(|| {
        assert_query_unbound(&r#loop::query_binding());
        let outcome = r#loop::present_ai_assistant_core().expect("Present must succeed");
        assert_eq!(
            outcome.window_label, r#loop::WINDOW_LABEL,
            "Present maps to existing ai-assistant shell focus surface"
        );
        assert_eq!(
            outcome.surface, "Present",
            "semantic surface name is Present (not a Binding Contract op)"
        );
        assert_eq!(
            outcome.entry_id, "ai-assistant",
            "T3 PresentOutcome must carry entry_id for shell openEntry"
        );
        assert_query_unbound(&r#loop::query_binding());
        let err = r#loop::execute_binding().expect_err("Present alone must not enable execute");
        assert_eq!(err.as_code(), "rejected_unbound");
    });
}

#[test]
fn present_preserves_binding_state_unbound_and_bound() {
    with_sandbox(|| {
        // unbound → Present → still unbound
        assert_query_unbound(&r#loop::query_binding());
        r#loop::present_ai_assistant_core().expect("Present unbound");
        assert_query_unbound(&r#loop::query_binding());

        // bound → Present → still bound (same generation)
        r#loop::set_binding(valid_binding()).expect("Set");
        let before = r#loop::query_binding();
        assert_query_bound(&before);
        r#loop::clear_lifecycle_events_for_tests();
        r#loop::present_ai_assistant_core().expect("Present while bound");
        let after = r#loop::query_binding();
        assert_eq!(
            after, before,
            "Present must not change query state while bound"
        );
        let events = r#loop::drain_lifecycle_events();
        assert!(
            events.is_empty(),
            "Present must not emit Binding lifecycle events: {events:?}"
        );
    });
}

#[test]
fn present_path_does_not_write_contract_business_binding_primary_key() {
    with_sandbox(|| {
        let master = create_bound_plan("Present无契约业务主键");
        // Present core must not smear a business primary key into Binding Contract state.
        let _ = r#loop::present_ai_assistant_core().expect("Present");
        assert_query_unbound(&r#loop::query_binding());
        assert_query_is_business_agnostic(&r#loop::query_binding());

        // Legacy open may still carry a master id for shell UX, but must not imply Set/bound.
        let _ = r#loop::open_ai_assistant_core(&master);
        assert_eq!(
            r#loop::binding_state(),
            "unbound",
            "open/Present path must not imply Binding Contract bound"
        );
        let q = r#loop::query_binding();
        assert_query_is_business_agnostic(&q);
        let encoded = serde_json::to_value(&q).unwrap();
        assert!(
            !encoded
                .as_object()
                .unwrap()
                .keys()
                .any(|k| k.contains("master_task")),
            "contract query must not expose business binding primary key: {encoded}"
        );
    });
}

#[test]
fn shell_close_is_not_reset_and_does_not_emit_on_unbound() {
    with_sandbox(|| {
        r#loop::set_binding(valid_binding()).expect("Set");
        let before = r#loop::query_binding();
        assert_query_bound(&before);
        r#loop::clear_lifecycle_events_for_tests();

        r#loop::shell_close_core().expect("shell close (关壳) must be callable");
        let after = r#loop::query_binding();
        assert_eq!(
            after, before,
            "关壳 ≠ Reset: binding state must stay bound until explicit Reset"
        );
        let events = r#loop::drain_lifecycle_events();
        assert!(
            events.iter().all(|e| e.event != "onUnbound"),
            "关壳 must not emit onUnbound; Reset must be explicit: {events:?}"
        );

        // Explicit Reset still works after shell close.
        r#loop::reset_binding().expect("explicit Reset");
        assert_query_unbound(&r#loop::query_binding());
    });
}

#[test]
fn present_command_json_does_not_set_binding() {
    with_sandbox(|| {
        use crate::commands::ai_assistant::{
            execute_binding_json, present_ai_assistant_json, query_binding_json, set_binding_json,
        };
        let presented = present_ai_assistant_json().expect("Present command");
        assert_eq!(presented["surface"], "Present");
        assert_eq!(presented["window_label"], r#loop::WINDOW_LABEL);
        assert_eq!(presented["entry_id"], "ai-assistant");
        let q = query_binding_json();
        assert_eq!(q["state"], "unbound");
        let exec = execute_binding_json();
        assert_eq!(exec["ok"], false);
        assert_eq!(exec["code"], "rejected_unbound");

        // Present after Set still leaves bound and does not replace.
        use crate::services::mcp_server_registry::SEEDED_BUSINESS_KEY;
        let set = set_binding_json(json!({ "key": SEEDED_BUSINESS_KEY }));
        assert_eq!(set["ok"], true);
        let gen_before = query_binding_json()["generation"].clone();
        let _ = present_ai_assistant_json().expect("Present while bound");
        assert_eq!(query_binding_json()["state"], "bound");
        assert_eq!(query_binding_json()["generation"], gen_before);
    });
}

// --- J1 / execute 门闩内核验收夹具（SK-4 / H1：无业务 UI 驱动）---
// 通用 Binding 夹具（tools+prompt+callbacks；callbacks 可空表）驱动全部 J1 场景。

fn j1_generic_binding_fixture(prompt: &str) -> r#loop::Binding {
    // Explicit generic fixture — no business page / 角位 / window-click driver.
    r#loop::Binding {
        tools: json!([{ "name": "fixture_tool", "handle": "opaque-fixture-tool" }]),
        prompt: json!(prompt),
        callbacks: json!({}), // empty registry allowed when slot present
    }
}

#[test]
fn j1_1_legal_set_on_bound_execute_reset_rejects() {
    with_sandbox(|| {
        r#loop::clear_lifecycle_events_for_tests();
        r#loop::set_binding(j1_generic_binding_fixture("j1-1-prompt")).expect("Set legal");
        let events_set = r#loop::drain_lifecycle_events();
        assert_eq!(event_names(&events_set), vec!["onBound"]);
        assert_payload_contract_only(&events_set[0]);

        let out = r#loop::execute_binding().expect("execute while bound");
        assert_eq!(out.applied_prompt, json!("j1-1-prompt"));
        assert_eq!(
            out.applied_tools,
            json!([{ "name": "fixture_tool", "handle": "opaque-fixture-tool" }])
        );

        r#loop::clear_lifecycle_events_for_tests();
        r#loop::reset_binding().expect("Reset");
        let events_reset = r#loop::drain_lifecycle_events();
        assert_eq!(event_names(&events_reset), vec!["onUnbound"]);
        assert_eq!(r#loop::binding_state(), "unbound");

        r#loop::clear_lifecycle_events_for_tests();
        let err = r#loop::execute_binding().expect_err("execute after Reset must reject");
        assert_eq!(err.as_code(), "rejected_unbound");
        let events_rej = r#loop::drain_lifecycle_events();
        let err_ev = events_rej
            .iter()
            .find(|e| e.event == "onError")
            .expect("onError(rejected_unbound)");
        assert_eq!(err_ev.category, Some("rejected_unbound"));
        assert_payload_contract_only(err_ev);
    });
}

#[test]
fn j1_2_illegal_set_keeps_state_no_on_bound_emits_set_invalid() {
    with_sandbox(|| {
        assert_eq!(r#loop::binding_state(), "unbound");
        r#loop::clear_lifecycle_events_for_tests();

        // Missing content / empty tools — B1 illegal Set.
        let err = r#loop::try_set_binding_json(&json!({
            "tools": [],
            "prompt": "p",
            "callbacks": {}
        }))
        .expect_err("illegal Set must fail");
        assert_eq!(err.as_code(), "set_invalid");
        assert_eq!(r#loop::binding_state(), "unbound");

        let events = r#loop::drain_lifecycle_events();
        assert!(
            events.iter().all(|e| e.event != "onBound"),
            "illegal Set must not emit onBound: {events:?}"
        );
        let err_ev = events
            .iter()
            .find(|e| e.event == "onError")
            .expect("J1-(2): onError(set_invalid) must be observable");
        assert_eq!(err_ev.category, Some("set_invalid"));
        assert_payload_contract_only(err_ev);

        // After bound, illegal Set must keep prior state and still emit set_invalid.
        r#loop::set_binding(j1_generic_binding_fixture("keep")).expect("seed");
        let gen = r#loop::query_binding().generation;
        r#loop::clear_lifecycle_events_for_tests();
        let err = r#loop::try_set_binding_json(&json!({
            "tools": [{ "name": "t" }],
            "prompt": "p"
            // callbacks missing
        }))
        .expect_err("missing slot");
        assert_eq!(err.as_code(), "set_invalid");
        assert_eq!(r#loop::binding_state(), "bound");
        assert_eq!(r#loop::query_binding().generation, gen);
        let events2 = r#loop::drain_lifecycle_events();
        assert!(events2.iter().all(|e| e.event != "onBound"));
        assert!(
            events2
                .iter()
                .any(|e| e.event == "onError" && e.category == Some("set_invalid")),
            "{events2:?}"
        );
    });
}

#[test]
fn j1_3_replace_set_on_unbound_then_on_bound_execute_uses_new() {
    with_sandbox(|| {
        r#loop::set_binding(j1_generic_binding_fixture("old-j1-3")).expect("first");
        let old_gen = r#loop::query_binding().generation.expect("old");
        r#loop::clear_lifecycle_events_for_tests();

        r#loop::set_binding(j1_generic_binding_fixture("new-j1-3")).expect("replace");
        let events = r#loop::drain_lifecycle_events();
        assert_eq!(event_names(&events), vec!["onUnbound", "onBound"]);
        assert!(!r#loop::is_binding_generation_current(old_gen));

        let out = r#loop::execute_binding().expect("execute uses new");
        assert_eq!(out.applied_prompt, json!("new-j1-3"));
        assert_ne!(out.applied_prompt, json!("old-j1-3"));
    });
}

#[test]
fn j1_4_mid_execute_reset_unbounds_cancels_with_on_error() {
    with_sandbox(|| {
        r#loop::set_binding(j1_generic_binding_fixture("in-flight-j1-4")).expect("Set");
        let gen = r#loop::query_binding().generation.expect("gen");
        r#loop::clear_lifecycle_events_for_tests();

        let err = r#loop::execute_binding_during(|| {
            r#loop::reset_binding().expect("Reset mid-execute");
            assert_eq!(r#loop::binding_state(), "unbound");
            assert!(!r#loop::is_binding_generation_current(gen));
        })
        .expect_err("must cancel");
        assert_eq!(err.as_code(), "reset_cancelled");

        let events = r#loop::drain_lifecycle_events();
        assert!(events.iter().any(|e| e.event == "onUnbound"), "{events:?}");
        let err_ev = events
            .iter()
            .find(|e| e.event == "onError")
            .expect("onError(reset_cancelled)");
        assert_eq!(err_ev.category, Some("reset_cancelled"));
    });
}

#[test]
fn j1_5_contract_states_tools_prompt_callbacks_present_not_bound() {
    with_sandbox(|| {
        let binding = j1_generic_binding_fixture("j1-5");
        let encoded = serde_json::to_value(&binding).unwrap();
        let obj = encoded.as_object().unwrap();
        assert!(obj.contains_key("tools"));
        assert!(obj.contains_key("prompt"));
        assert!(obj.contains_key("callbacks"));
        assert!(!obj.contains_key("master_task_id"));
        assert!(!obj.contains_key("bound_master_task_id"));
        assert!(!obj.contains_key("todo_id"));

        // Present / 壳打开 ≠ bound
        let _ = r#loop::present_ai_assistant_core().expect("Present");
        assert_eq!(r#loop::binding_state(), "unbound");
        let err = r#loop::execute_binding().expect_err("Present alone ≠ bound");
        assert_eq!(err.as_code(), "rejected_unbound");

        let ops = r#loop::binding_contract_ops();
        let lower: Vec<_> = ops.iter().map(|s| s.to_ascii_lowercase()).collect();
        assert!(lower.iter().any(|o| o == "set"));
        assert!(lower.iter().any(|o| o == "reset"));
        assert!(lower.iter().any(|o| o == "query"));
        assert!(lower.iter().any(|o| o == "execute"));
        assert!(lower.iter().any(|o| o.contains("callback")));
        assert!(lower.iter().all(|o| o != "present" && !o.contains("present")));
        assert!(lower.iter().all(|o| o != "open" && !o.contains("open")));
    });
}

#[test]
fn j1_execute_gate_bound_success_and_unbound_reject() {
    with_sandbox(|| {
        // unbound → reject + onError(rejected_unbound)
        assert_eq!(r#loop::binding_state(), "unbound");
        r#loop::clear_lifecycle_events_for_tests();
        let err = r#loop::execute_binding().expect_err("unbound hard-fail");
        assert_eq!(err.as_code(), "rejected_unbound");
        assert!(
            r#loop::drain_lifecycle_events()
                .iter()
                .any(|e| e.event == "onError" && e.category == Some("rejected_unbound"))
        );

        // bound → success with current Binding as sole config
        r#loop::set_binding(j1_generic_binding_fixture("gate-ok")).expect("Set");
        let out = r#loop::execute_binding().expect("bound success");
        assert_eq!(out.applied_prompt, json!("gate-ok"));
        assert_eq!(r#loop::binding_state(), "bound");
    });
}

#[test]
fn j1_h1_kernel_api_fixture_driver_not_business_ui() {
    with_sandbox(|| {
        // H1: acceptance driven by kernel API + generic Binding fixture only.
        // Prove Present observation does not substitute for contract Set.
        let _ = r#loop::present_ai_assistant_core().expect("Present observable");
        assert_eq!(r#loop::binding_state(), "unbound");

        r#loop::set_binding(j1_generic_binding_fixture("h1-kernel")).expect("kernel Set");
        assert_eq!(r#loop::binding_state(), "bound");
        let _ = r#loop::execute_binding().expect("kernel execute");
        r#loop::reset_binding().expect("kernel Reset");
        assert_eq!(r#loop::binding_state(), "unbound");

        // No business-page entry point invoked; fixture callbacks remain empty registry.
        let encoded = serde_json::to_value(j1_generic_binding_fixture("h1-kernel")).unwrap();
        assert_eq!(encoded["callbacks"], json!({}));
        assert!(encoded.get("master_task_id").is_none());
    });
}

// --- T1: production path binding-generation gate ---

/// Mock LLM that runs `mid` (e.g. Reset / replace Set) before returning a scripted response.
fn spawn_llm_with_mid_then_response<F>(mid: F, response: (u16, Value)) -> MockLlm
where
    F: FnOnce() + Send + 'static,
{
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().unwrap().port();
    let hits = Arc::new(Mutex::new(Vec::new()));
    let hits_t = hits.clone();
    let mid = Arc::new(Mutex::new(Some(mid)));
    let join = thread::spawn(move || {
        let Ok((mut stream, _)) = listener.accept() else {
            return;
        };
        let mut buf = [0u8; 65536];
        let n = stream.read(&mut buf).unwrap_or(0);
        let raw = String::from_utf8_lossy(&buf[..n]);
        let body_str = raw.split("\r\n\r\n").nth(1).unwrap_or("");
        let body: Value = serde_json::from_str(body_str.trim_end_matches('\0').trim())
            .unwrap_or(json!({}));
        hits_t.lock().unwrap().push(body);
        if let Some(f) = mid.lock().unwrap().take() {
            f();
        }
        let (status, resp) = response;
        let body = resp.to_string();
        let resp = format!(
            "HTTP/1.1 {status} OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        let _ = stream.write_all(resp.as_bytes());
    });
    thread::sleep(Duration::from_millis(20));
    MockLlm {
        port,
        hits,
        _join: join,
    }
}

#[test]
fn t1_execute_when_bound_and_generation_current_succeeds() {
    with_sandbox(|| {
        r#loop::set_binding(binding_with_prompt("gen-ok")).expect("Set");
        let gen = r#loop::query_binding().generation.expect("gen");
        assert!(
            r#loop::is_binding_generation_current(gen),
            "execute path requires a live generation"
        );
        let out = r#loop::execute_binding().expect("bound+current gen must succeed");
        assert_eq!(out.applied_prompt, json!("gen-ok"));
        assert!(r#loop::is_binding_generation_current(gen));
    });
}

#[test]
fn t1_replace_set_advances_generation_then_execute_under_new() {
    with_sandbox(|| {
        r#loop::set_binding(binding_with_prompt("old-gen")).expect("first");
        let old_gen = r#loop::query_binding().generation.expect("old");
        r#loop::set_binding(binding_with_prompt("new-gen")).expect("replace");
        let new_gen = r#loop::query_binding().generation.expect("new");
        assert_ne!(old_gen, new_gen);
        assert!(!r#loop::is_binding_generation_current(old_gen));
        assert!(r#loop::is_binding_generation_current(new_gen));
        let out = r#loop::execute_binding().expect("execute under new generation");
        assert_eq!(out.applied_prompt, json!("new-gen"));
        assert_ne!(out.applied_prompt, json!("old-gen"));
    });
}

#[test]
fn t1_mid_execute_replace_set_rejects_stale_generation() {
    with_sandbox(|| {
        r#loop::set_binding(binding_with_prompt("in-flight-old")).expect("Set");
        let old_gen = r#loop::query_binding().generation.expect("old gen");
        r#loop::clear_lifecycle_events_for_tests();

        let err = r#loop::execute_binding_during(|| {
            r#loop::set_binding(binding_with_prompt("in-flight-new")).expect("replace mid-execute");
            assert!(
                !r#loop::is_binding_generation_current(old_gen),
                "replace must invalidate the snapshotted generation"
            );
        })
        .expect_err("in-flight execute must reject stale generation");
        assert_eq!(
            err.as_code(),
            "rejected_stale_generation",
            "Host must return a distinguishable generation-stale reject reason"
        );
        assert_eq!(r#loop::binding_state(), "bound");
        let new_gen = r#loop::query_binding().generation.expect("new gen");
        assert!(r#loop::is_binding_generation_current(new_gen));
        assert_ne!(old_gen, new_gen);

        let events = r#loop::drain_lifecycle_events();
        assert!(
            events.iter().any(|e| {
                e.event == "onError" && e.category == Some("rejected_stale_generation")
            }),
            "onError(rejected_stale_generation) must be observable: {events:?}"
        );
        // Old Tools/Prompt must not win; a fresh execute under the new generation applies new prompt.
        let out = r#loop::execute_binding().expect("fresh execute under new gen");
        assert_eq!(out.applied_prompt, json!("in-flight-new"));
    });
}

#[test]
fn t1_query_and_present_do_not_authorize_execute() {
    with_sandbox(|| {
        assert_eq!(r#loop::binding_state(), "unbound");
        let _ = r#loop::query_binding();
        let _ = r#loop::present_ai_assistant_core().expect("Present");
        assert_eq!(r#loop::binding_state(), "unbound");
        let err = r#loop::execute_binding().expect_err("query/Present alone must not authorize");
        assert_eq!(err.as_code(), "rejected_unbound");
    });
}

#[test]
fn t1_agent_chat_turn_allows_when_generation_current() {
    with_sandbox(|| {
        let master = create_bound_plan("t1-gen-ok");
        let mock = spawn_scripted_llm(vec![assistant_text("世代有效")]);
        install_llm_cfg(&mock);
        arm_plan_binding(&master);
        let gen = r#loop::query_binding().generation.expect("gen");
        assert!(r#loop::is_binding_generation_current(gen));
        let open = r#loop::open_ai_assistant_core(&master).unwrap();
        let sid = open["session_id"].as_str().unwrap();
        let result = r#loop::agent_chat_turn_core(sid, "你好", Some(&master)).unwrap();
        assert_eq!(result.body["terminal"], "none");
        assert!(result.body["reply_text"].as_str().unwrap().contains("世代有效"));
        assert_eq!(mock.hits.lock().unwrap().len(), 1);
    });
}

#[test]
fn t1_agent_chat_turn_after_reset_does_not_invoke_llm() {
    with_sandbox(|| {
        let master = create_bound_plan("t1-gen-reset");
        let mock = spawn_scripted_llm(vec![assistant_text("should-not-run")]);
        install_llm_cfg(&mock);
        arm_plan_binding(&master);
        let gen = r#loop::query_binding().generation.expect("gen");
        let open = r#loop::open_ai_assistant_core(&master).unwrap();
        let sid = open["session_id"].as_str().unwrap().to_string();
        r#loop::reset_binding().expect("Reset");
        assert!(!r#loop::is_binding_generation_current(gen));

        let result = r#loop::agent_chat_turn_core(&sid, "继续", Some(&master)).unwrap();
        assert_ne!(
            result.body["terminal"],
            "none",
            "after Reset, chat must not continue as a successful executable turn"
        );
        assert_eq!(
            mock.hits.lock().unwrap().len(),
            0,
            "generation invalidation must not call LLM / old Tools+Prompt path"
        );
    });
}

#[test]
fn t1_run_loop_aborts_tool_dispatch_when_generation_invalidated() {
    with_sandbox(|| {
        let master = create_bound_plan("t1-gen-tools");
        let title_before = todo_task::get_by_id(&master)["title"]
            .as_str()
            .unwrap()
            .to_string();
        let gen_holder = Arc::new(Mutex::new(None::<u64>));
        let gen_holder_c = gen_holder.clone();
        let mock = spawn_llm_with_mid_then_response(
            move || {
                let gen = gen_holder_c.lock().unwrap().expect("gen snapshotted");
                r#loop::reset_binding().expect("Reset during LLM / before tool dispatch");
                assert!(!r#loop::is_binding_generation_current(gen));
            },
            assistant_tools(
                json!([{
                    "id": "t1c1",
                    "type": "function",
                    "function": {
                        "name": "update_master_title",
                        "arguments": "{\"title\":\"不得写入旧世代\"}"
                    }
                }]),
                None,
            ),
        );
        install_llm_cfg(&mock);
        arm_plan_binding(&master);
        let gen = r#loop::query_binding().generation.expect("gen");
        *gen_holder.lock().unwrap() = Some(gen);
        let open = r#loop::open_ai_assistant_core(&master).unwrap();
        let sid = open["session_id"].as_str().unwrap();

        let result = r#loop::agent_chat_turn_core(sid, "改标题", Some(&master)).unwrap();
        assert_eq!(
            result.body["wrote"],
            false,
            "tool dispatch must not proceed on stale generation"
        );
        assert_eq!(
            result.body["terminal"],
            "error",
            "stale-generation abort must be an error terminal, not a successful turn"
        );
        assert_eq!(
            todo_task::get_by_id(&master)["title"],
            title_before,
            "old Tools must not write after generation invalidation"
        );
    });
}

// --- T2: session cut — clear-first (current_session_id → None) ---

fn live_session_id() -> Option<String> {
    let sid = r#loop::get_ai_assistant_binding_core()["session_id"]
        .as_str()
        .unwrap_or("")
        .to_string();
    if sid.is_empty() {
        None
    } else {
        Some(sid)
    }
}

#[test]
fn t2_reset_clears_current_session_id_to_none() {
    with_sandbox(|| {
        let master = create_bound_plan("t2-reset-clear");
        arm_plan_binding(&master);
        let open = r#loop::open_ai_assistant_core(&master).unwrap();
        let old_sid = open["session_id"].as_str().unwrap().to_string();
        assert_eq!(live_session_id().as_deref(), Some(old_sid.as_str()));

        r#loop::reset_binding().expect("Reset");
        assert_eq!(
            live_session_id(),
            None,
            "Reset must clear current_session_id to None (clear-first)"
        );
        assert_eq!(r#loop::binding_state(), "unbound");
    });
}

#[test]
fn t2_reset_old_session_not_reused_by_ensure_for_executable() {
    with_sandbox(|| {
        let master = create_bound_plan("t2-reset-ensure");
        arm_plan_binding(&master);
        let open = r#loop::open_ai_assistant_core(&master).unwrap();
        let old_sid = open["session_id"].as_str().unwrap().to_string();
        // Seed turns on the old session so reuse would be observable.
        let mut old = session::load_session(&old_sid).unwrap();
        old.turns.push(Turn {
            role: "user".into(),
            content: Some("old-turn".into()),
            tool_call_id: None,
            tool_calls: None,
            name: None,
        });
        session::save_session(&old).unwrap();

        r#loop::reset_binding().expect("Reset");
        assert_eq!(live_session_id(), None);

        // Re-bind then ensure: must mint a new session, not reuse the cut id.
        arm_plan_binding(&master);
        let ensured = r#loop::ensure_chat_session_core().expect("ensure after cut");
        let new_sid = ensured["session_id"].as_str().unwrap().to_string();
        assert_ne!(
            new_sid, old_sid,
            "ensure must not reuse a cut session id for executable chat"
        );
        assert_eq!(live_session_id().as_deref(), Some(new_sid.as_str()));
        let fresh = session::load_session(&new_sid).unwrap();
        assert!(
            fresh.turns.is_empty(),
            "new executable session must not carry old turns"
        );
    });
}

#[test]
fn t2_replace_set_clears_session_with_generation_advance() {
    with_sandbox(|| {
        let master_a = create_bound_plan("t2-replace-a");
        let master_b = create_bound_plan("t2-replace-b");
        arm_plan_binding(&master_a);
        let open = r#loop::open_ai_assistant_core(&master_a).unwrap();
        let old_sid = open["session_id"].as_str().unwrap().to_string();
        let old_gen = r#loop::query_binding().generation.expect("old gen");
        assert_eq!(live_session_id().as_deref(), Some(old_sid.as_str()));

        arm_plan_binding(&master_b); // successful replace Set
        let new_gen = r#loop::query_binding().generation.expect("new gen");
        assert_ne!(old_gen, new_gen, "replace Set must advance generation");
        assert!(
            !r#loop::is_binding_generation_current(old_gen),
            "old generation must be invalid"
        );
        assert_eq!(
            live_session_id(),
            None,
            "replace Set must clear current_session_id (same semantics as Reset); \
             must not leave a new-generation + old-session window"
        );
    });
}

#[test]
fn t2_first_set_clears_pre_set_session_then_ensure_mints_new() {
    with_sandbox(|| {
        // unbound → ensure mints a session that must not auto-promote after Set.
        let pre = r#loop::ensure_chat_session_core().expect("ensure while unbound");
        let pre_sid = pre["session_id"].as_str().unwrap().to_string();
        assert_eq!(live_session_id().as_deref(), Some(pre_sid.as_str()));
        assert_eq!(r#loop::binding_state(), "unbound");

        let master = create_bound_plan("t2-first-set");
        arm_plan_binding(&master); // unbound→bound first successful Set
        assert_eq!(
            live_session_id(),
            None,
            "first successful Set must clear any pre-Set current_session_id"
        );
        assert_ne!(
            r#loop::query_binding().generation,
            None,
            "first Set must establish a live generation"
        );

        let ensured = r#loop::ensure_chat_session_core().expect("ensure after first Set");
        let new_sid = ensured["session_id"].as_str().unwrap().to_string();
        assert_ne!(
            new_sid, pre_sid,
            "unbound-era ensure session must not become the new binding executable context"
        );
    });
}

#[test]
fn t2_re_set_executable_chat_lands_on_new_session_without_old_turns() {
    with_sandbox(|| {
        let master_a = create_bound_plan("t2-re-set-a");
        let master_b = create_bound_plan("t2-re-set-b");
        let mock = spawn_scripted_llm(vec![
            assistant_text("旧绑定回复"),
            assistant_text("新绑定回复"),
        ]);
        install_llm_cfg(&mock);

        arm_plan_binding(&master_a);
        let open = r#loop::open_ai_assistant_core(&master_a).unwrap();
        let old_sid = open["session_id"].as_str().unwrap().to_string();
        let first = r#loop::agent_chat_turn_core(&old_sid, "你好", Some(&master_a)).unwrap();
        assert_eq!(first.body["terminal"], "none");
        let old_turns_len = session::load_session(&old_sid).unwrap().turns.len();
        assert!(old_turns_len >= 2, "old session should have turns");

        arm_plan_binding(&master_b); // replace Set → session cut
        assert_eq!(live_session_id(), None);

        let ensured = r#loop::ensure_chat_session_core().expect("ensure new");
        let new_sid = ensured["session_id"].as_str().unwrap().to_string();
        assert_ne!(new_sid, old_sid);
        let result = r#loop::agent_chat_turn_core(&new_sid, "继续", Some(&master_b)).unwrap();
        assert_eq!(result.body["terminal"], "none");
        assert!(
            result.body["reply_text"]
                .as_str()
                .unwrap()
                .contains("新绑定回复")
        );
        let new_sess = session::load_session(&new_sid).unwrap();
        let blob = serde_json::to_string(&new_sess.turns).unwrap();
        assert!(
            !blob.contains("旧绑定回复") && !blob.contains("你好"),
            "new binding executable session must not carry old turns: {blob}"
        );
        // Old session file may still exist with its turns (cut ≠ delete).
        let old_still = session::load_session(&old_sid).unwrap();
        assert_eq!(old_still.turns.len(), old_turns_len);
    });
}

#[test]
fn t2_unbound_ensure_session_not_auto_promoted_on_set() {
    with_sandbox(|| {
        let pre = r#loop::ensure_chat_session_core().expect("unbound ensure");
        let pre_sid = pre["session_id"].as_str().unwrap().to_string();
        let mut seeded = session::load_session(&pre_sid).unwrap();
        seeded.turns.push(Turn {
            role: "user".into(),
            content: Some("unbound-era".into()),
            tool_call_id: None,
            tool_calls: None,
            name: None,
        });
        session::save_session(&seeded).unwrap();

        r#loop::set_binding(valid_binding()).expect("Set");
        assert_eq!(live_session_id(), None);

        let after = r#loop::ensure_chat_session_core().expect("ensure under new binding");
        let after_sid = after["session_id"].as_str().unwrap().to_string();
        assert_ne!(after_sid, pre_sid);
        let after_sess = session::load_session(&after_sid).unwrap();
        assert!(
            after_sess
                .turns
                .iter()
                .all(|t| t.content.as_deref() != Some("unbound-era")),
            "unbound-ensure session must not auto-promote into new binding context"
        );
    });
}

#[test]
fn t2_chat_session_identity_must_match_live_current() {
    with_sandbox(|| {
        let master = create_bound_plan("t2-identity");
        let mock = spawn_scripted_llm(vec![assistant_text("should-not-run")]);
        install_llm_cfg(&mock);
        arm_plan_binding(&master);
        let open = r#loop::open_ai_assistant_core(&master).unwrap();
        let live = open["session_id"].as_str().unwrap().to_string();
        assert_eq!(live_session_id().as_deref(), Some(live.as_str()));

        // Stale / foreign session id must be rejected; must not re-pin runtime.
        let foreign = session::create_session(None, None).unwrap().session_id;
        let rejected = r#loop::agent_chat_turn_core(&foreign, "ping", Some(&master));
        match rejected {
            Ok(result) => {
                assert_ne!(
                    result.body["terminal"],
                    "none",
                    "mismatched session id must not continue as a successful executable turn"
                );
            }
            Err(_) => {} // hard reject is also acceptable
        }
        assert_eq!(
            live_session_id().as_deref(),
            Some(live.as_str()),
            "reject must not re-pin runtime current_session_id to the foreign id"
        );
        assert_eq!(mock.hits.lock().unwrap().len(), 0);
    });
}

#[test]
fn t2_cut_does_not_wipe_turns_as_primary_means() {
    with_sandbox(|| {
        let master = create_bound_plan("t2-no-wipe");
        arm_plan_binding(&master);
        let open = r#loop::open_ai_assistant_core(&master).unwrap();
        let old_sid = open["session_id"].as_str().unwrap().to_string();
        let mut sess = session::load_session(&old_sid).unwrap();
        sess.turns.push(Turn {
            role: "user".into(),
            content: Some("preserve-me".into()),
            tool_call_id: None,
            tool_calls: None,
            name: None,
        });
        session::save_session(&sess).unwrap();
        let turns_before = sess.turns.len();

        r#loop::reset_binding().expect("Reset");
        assert_eq!(live_session_id(), None);

        // Primary cut means clearing the live id — not wiping turns on the same id.
        let still = session::load_session(&old_sid).expect("disk json may remain");
        assert_eq!(
            still.turns.len(),
            turns_before,
            "cut must not use same-id turn wipe as the primary means"
        );
        assert!(
            still
                .turns
                .iter()
                .any(|t| t.content.as_deref() == Some("preserve-me"))
        );

        // Old id must not drive executable chat after cut.
        let mock = spawn_scripted_llm(vec![assistant_text("should-not-run")]);
        install_llm_cfg(&mock);
        // Still unbound after Reset — re-bind so generation gate isn't the only rejector.
        arm_plan_binding(&master);
        assert_eq!(
            live_session_id(),
            None,
            "re-Set after Reset must also leave no live session (first/replace Set clears)"
        );
        let rejected = r#loop::agent_chat_turn_core(&old_sid, "drive-old", Some(&master));
        match rejected {
            Ok(result) => assert_ne!(result.body["terminal"], "none"),
            Err(_) => {}
        }
        assert_eq!(
            live_session_id(),
            None,
            "calling with cut id must not re-pin runtime to the cut session"
        );
        assert_eq!(mock.hits.lock().unwrap().len(), 0);
    });
}

// --- T3: symmetric in-flight cancel for execute + chat ---

fn session_turn_contents(sid: &str) -> Vec<Option<String>> {
    session::load_session(sid)
        .unwrap()
        .turns
        .iter()
        .map(|t| t.content.clone())
        .collect()
}

#[test]
fn t3_reset_while_busy_and_executing_cancels_both_inflight_paths() {
    with_sandbox(|| {
        r#loop::set_binding(valid_binding()).expect("Set");
        r#loop::set_busy_for_tests(true);
        let err = r#loop::execute_binding_during(|| {
            assert_eq!(
                r#loop::get_ai_assistant_binding_core()["busy"],
                true,
                "chat in-flight must be observable as busy"
            );
            r#loop::reset_binding().expect("Reset");
            assert!(
                r#loop::is_execute_cancelled_for_tests(),
                "Reset must set execute cancel while executing"
            );
            assert!(
                r#loop::is_chat_cancelled_for_tests(),
                "Reset must set chat cancel while busy — not execute-only"
            );
        })
        .expect_err("in-flight execute must cancel");
        assert_eq!(err.as_code(), "reset_cancelled");
        assert_eq!(r#loop::binding_state(), "unbound");
        assert_eq!(live_session_id(), None);
    });
}

#[test]
fn t3_replace_set_while_busy_cancels_chat_and_interrupts_execute() {
    with_sandbox(|| {
        r#loop::set_binding(valid_binding()).expect("Set");
        r#loop::set_busy_for_tests(true);
        let old_gen = r#loop::query_binding().generation.expect("old gen");
        let err = r#loop::execute_binding_during(|| {
            r#loop::set_binding(binding_with_prompt("replaced-mid-flight")).expect("replace Set");
            assert!(
                !r#loop::is_binding_generation_current(old_gen),
                "replace Set must invalidate the in-flight execute generation"
            );
            assert!(
                r#loop::is_chat_cancelled_for_tests(),
                "replace Set must cancel in-flight chat while busy — not execute-only"
            );
            assert_eq!(live_session_id(), None);
        })
        .expect_err("in-flight execute must be interrupted by replace Set");
        // Generation-stale remains the distinguishable replace reject (T1); interrupt is required.
        assert_eq!(err.as_code(), "rejected_stale_generation");
    });
}

#[test]
fn t3_mid_chat_reset_cancels_without_appending_business_turns() {
    with_sandbox(|| {
        let master = create_bound_plan("t3-chat-reset");
        let title_before = todo_task::get_by_id(&master)["title"]
            .as_str()
            .unwrap()
            .to_string();
        let mock = spawn_llm_with_mid_then_response(
            || {
                assert_eq!(
                    r#loop::get_ai_assistant_binding_core()["busy"],
                    true,
                    "in-flight chat must set busy before LLM returns"
                );
                r#loop::reset_binding().expect("Reset during chat");
                assert!(
                    r#loop::is_chat_cancelled_for_tests(),
                    "Reset must raise chat cancel flag"
                );
                assert_eq!(live_session_id(), None, "cut clears session immediately");
            },
            assistant_tools(
                json!([{
                    "id": "t3c1",
                    "type": "function",
                    "function": {
                        "name": "update_master_title",
                        "arguments": "{\"title\":\"cancelled-must-not-write\"}"
                    }
                }]),
                None,
            ),
        );
        install_llm_cfg(&mock);
        arm_plan_binding(&master);
        let open = r#loop::open_ai_assistant_core(&master).unwrap();
        let sid = open["session_id"].as_str().unwrap().to_string();
        let turns_before = session_turn_contents(&sid);

        let result = r#loop::agent_chat_turn_core(&sid, "改标题", Some(&master)).unwrap();
        assert_eq!(result.body["wrote"], false);
        assert_eq!(
            result.body["terminal"], "error",
            "cancelled in-flight chat must stop as error, not fly the round to success"
        );
        assert_eq!(
            todo_task::get_by_id(&master)["title"],
            title_before,
            "cancelled chat must not dispatch tools"
        );
        assert_eq!(
            session_turn_contents(&sid),
            turns_before,
            "Host cancel reject must not append/persist business turns on the old session"
        );
        assert_eq!(r#loop::binding_state(), "unbound");
        assert_eq!(live_session_id(), None);
    });
}

#[test]
fn t3_mid_chat_replace_set_cancels_without_appending_business_turns() {
    with_sandbox(|| {
        let master_a = create_bound_plan("t3-chat-replace-a");
        let master_b = create_bound_plan("t3-chat-replace-b");
        let title_before = todo_task::get_by_id(&master_a)["title"]
            .as_str()
            .unwrap()
            .to_string();
        let mock = spawn_llm_with_mid_then_response(
            move || {
                arm_plan_binding(&master_b);
                assert!(
                    r#loop::is_chat_cancelled_for_tests(),
                    "replace Set must raise chat cancel while busy"
                );
                assert_eq!(live_session_id(), None);
            },
            assistant_tools(
                json!([{
                    "id": "t3c2",
                    "type": "function",
                    "function": {
                        "name": "update_master_title",
                        "arguments": "{\"title\":\"replace-cancel-must-not-write\"}"
                    }
                }]),
                None,
            ),
        );
        install_llm_cfg(&mock);
        arm_plan_binding(&master_a);
        let open = r#loop::open_ai_assistant_core(&master_a).unwrap();
        let sid = open["session_id"].as_str().unwrap().to_string();
        let turns_before = session_turn_contents(&sid);

        let result = r#loop::agent_chat_turn_core(&sid, "改标题", Some(&master_a)).unwrap();
        assert_eq!(result.body["wrote"], false);
        assert_eq!(result.body["terminal"], "error");
        assert_eq!(todo_task::get_by_id(&master_a)["title"], title_before);
        assert_eq!(
            session_turn_contents(&sid),
            turns_before,
            "replace-Set cancel must not append business turns to the cut session"
        );
        assert_eq!(live_session_id(), None);
    });
}

#[test]
fn t3_run_loop_checks_cancel_flag_before_tool_dispatch() {
    with_sandbox(|| {
        let master = create_bound_plan("t3-flag-gate");
        let title_before = todo_task::get_by_id(&master)["title"]
            .as_str()
            .unwrap()
            .to_string();
        let mock = spawn_llm_with_mid_then_response(
            || {
                // Raise chat cancel without Reset — proves run_loop checks the
                // cancel flag itself (not only generation invalidation).
                r#loop::set_chat_cancelled_for_tests(true);
            },
            assistant_tools(
                json!([{
                    "id": "t3c3",
                    "type": "function",
                    "function": {
                        "name": "update_master_title",
                        "arguments": "{\"title\":\"flag-cancel-must-not-write\"}"
                    }
                }]),
                None,
            ),
        );
        install_llm_cfg(&mock);
        arm_plan_binding(&master);
        let gen = r#loop::query_binding().generation.expect("gen");
        let open = r#loop::open_ai_assistant_core(&master).unwrap();
        let sid = open["session_id"].as_str().unwrap().to_string();
        let turns_before = session_turn_contents(&sid);

        let result = r#loop::agent_chat_turn_core(&sid, "改标题", Some(&master)).unwrap();
        assert!(
            r#loop::is_binding_generation_current(gen),
            "this case keeps generation current; stop must be from cancel flag"
        );
        assert_eq!(result.body["wrote"], false);
        assert_eq!(result.body["terminal"], "error");
        assert_eq!(todo_task::get_by_id(&master)["title"], title_before);
        assert_eq!(
            session_turn_contents(&sid),
            turns_before,
            "cancel-flag reject must not append business turns"
        );
    });
}

#[test]
fn t3_defensive_unbound_cancels_inflight_chat_symmetrically() {
    with_sandbox(|| {
        let master = create_bound_plan("t3-defensive");
        let mock = spawn_llm_with_mid_then_response(
            || {
                r#loop::defensive_unbound().expect("defensive cut");
                assert!(
                    r#loop::is_chat_cancelled_for_tests(),
                    "defensive cut must cancel in-flight chat"
                );
                assert_eq!(r#loop::binding_state(), "unbound");
                assert_eq!(live_session_id(), None);
            },
            assistant_text("should-not-land"),
        );
        install_llm_cfg(&mock);
        arm_plan_binding(&master);
        let open = r#loop::open_ai_assistant_core(&master).unwrap();
        let sid = open["session_id"].as_str().unwrap().to_string();
        let turns_before = session_turn_contents(&sid);

        let result = r#loop::agent_chat_turn_core(&sid, "你好", Some(&master)).unwrap();
        assert_eq!(result.body["terminal"], "error");
        assert_eq!(result.body["wrote"], false);
        assert_eq!(
            session_turn_contents(&sid),
            turns_before,
            "defensive cancel reject must not append business turns"
        );
        assert_eq!(mock.hits.lock().unwrap().len(), 1);
    });
}

#[test]
fn t3_cancel_notice_is_not_cut_semantics() {
    with_sandbox(|| {
        // Cut semantics = cancel flags + generation invalidate + session clear.
        // Returning a notice/error body is allowed; writing notice turns is not the cut.
        r#loop::set_binding(valid_binding()).expect("Set");
        r#loop::set_busy_for_tests(true);
        r#loop::reset_binding().expect("Reset");
        assert!(r#loop::is_chat_cancelled_for_tests());
        assert_eq!(r#loop::binding_state(), "unbound");
        assert_eq!(live_session_id(), None);
        assert!(
            r#loop::query_binding().generation.is_none(),
            "cut must invalidate generation; notice turns are not a substitute"
        );
    });
}

// --- T4: Host defensive cut (同语义 Reset; shell_close ≠ cut; 幂等) ---
// Must Close Before P2: DEFENSIVE_CUT_HOOK_PATH must be a stable, testable Host symbol.

#[test]
fn t4_defensive_cut_hook_path_is_confirmed_and_testable() {
    // Must Close Before: confirm hook-point file path is testable.
    assert_eq!(
        r#loop::DEFENSIVE_CUT_HOOK_PATH,
        "src-tauri/src/services/agent/loop.rs::defensive_unbound",
        "P2 Done requires a confirmed, testable Host defensive-cut hook path"
    );
    // Explicit leave→Reset remains the primary path; defensive cut backs missed leave.
    assert_eq!(
        r#loop::DEFENSIVE_CUT_EXPLICIT_RESET_CHAIN,
        [
            "frontend/js/plan-task/index.js::dispose",
            "frontend/js/plan-task/todos-lifecycle.js::onTodosPageLeave",
            "frontend/js/plan-task/todos-binding.js::resetTodosBinding",
            "src-tauri/src/services/agent/loop.rs::reset_binding",
        ]
    );
    // Hook symbol is callable (not a UI-only stub).
    with_sandbox(|| {
        r#loop::defensive_unbound().expect("hook must be invokable");
        assert_eq!(r#loop::binding_state(), "unbound");
    });
}

#[test]
fn t4_missed_reset_defensive_cut_matches_explicit_reset_semantics() {
    with_sandbox(|| {
        // Simulate「业务已退出仍 bound / 漏 Reset」: dispose leave signal missed,
        // Host still observes bound + live session → defensive_unbound.
        let master = create_bound_plan("t4-missed-reset");
        arm_plan_binding(&master);
        let open = r#loop::open_ai_assistant_core(&master).unwrap();
        let old_sid = open["session_id"].as_str().unwrap().to_string();
        let gen = r#loop::query_binding().generation.expect("gen");
        assert_eq!(r#loop::binding_state(), "bound");
        assert_eq!(live_session_id().as_deref(), Some(old_sid.as_str()));
        r#loop::set_busy_for_tests(true);
        r#loop::clear_lifecycle_events_for_tests();

        r#loop::defensive_unbound().expect("Host defensive cut on missed Reset");

        assert_eq!(r#loop::binding_state(), "unbound");
        assert_eq!(
            live_session_id(),
            None,
            "defensive cut must clear current_session_id (same as Reset)"
        );
        assert!(
            r#loop::query_binding().generation.is_none(),
            "defensive cut must invalidate generation"
        );
        assert!(
            !r#loop::is_binding_generation_current(gen),
            "old generation must not remain current"
        );
        assert!(
            r#loop::is_chat_cancelled_for_tests(),
            "defensive cut must cancel in-flight chat (symmetric with Reset)"
        );
        let events = r#loop::drain_lifecycle_events();
        assert_eq!(
            event_names(&events),
            vec!["onUnbound"],
            "defensive cut must emit onUnbound like explicit Reset: {events:?}"
        );

        // Old binding/session must not drive executable dialogue.
        let exec_err = r#loop::execute_binding().expect_err("execute after defensive cut");
        assert_eq!(exec_err.as_code(), "rejected_unbound");
        // Release artificial busy (no live chat turn owns it); cut already cleared live session.
        r#loop::set_busy_for_tests(false);
        let chat = r#loop::agent_chat_turn_core(&old_sid, "漏Reset后不可执行", Some(&master))
            .expect("chat path returns body");
        assert_eq!(
            chat.body["terminal"], "business",
            "stale/cut session must reject without executable continuation"
        );
        assert_eq!(chat.body["wrote"], false);
    });
}

#[test]
fn t4_defensive_cut_after_explicit_reset_is_idempotent() {
    with_sandbox(|| {
        let master = create_bound_plan("t4-idempotent");
        arm_plan_binding(&master);
        let _ = r#loop::open_ai_assistant_core(&master).unwrap();
        r#loop::set_busy_for_tests(true);

        // Explicit Reset (dispose→onTodosPageLeave→resetTodosBinding→reset_binding) succeeded.
        r#loop::reset_binding().expect("explicit Reset");
        assert_eq!(r#loop::binding_state(), "unbound");
        assert_eq!(live_session_id(), None);
        r#loop::clear_lifecycle_events_for_tests();

        // Defensive cut must be no-op / idempotent — no second onUnbound.
        r#loop::defensive_unbound().expect("defensive after Reset");
        assert_eq!(r#loop::binding_state(), "unbound");
        assert_eq!(live_session_id(), None);
        assert!(r#loop::query_binding().generation.is_none());
        let events = r#loop::drain_lifecycle_events();
        assert!(
            events.iter().all(|e| e.event != "onUnbound"),
            "idempotent defensive cut must not re-emit onUnbound: {events:?}"
        );

        r#loop::defensive_unbound().expect("second defensive still ok");
        assert_eq!(r#loop::binding_state(), "unbound");
    });
}

#[test]
fn t4_shell_close_core_is_not_defensive_cut() {
    with_sandbox(|| {
        let master = create_bound_plan("t4-shell-close");
        arm_plan_binding(&master);
        let open = r#loop::open_ai_assistant_core(&master).unwrap();
        let sid = open["session_id"].as_str().unwrap().to_string();
        let before = r#loop::query_binding();
        assert_query_bound(&before);
        let gen = before.generation.expect("gen");
        r#loop::clear_lifecycle_events_for_tests();

        // shell_close_core is defined only in loop.rs; shell_close_json only wraps it.
        r#loop::shell_close_core().expect("关壳");
        assert_eq!(
            r#loop::query_binding(),
            before,
            "关壳 ≠ 切断: binding must remain observable as before explicit Reset"
        );
        assert_eq!(live_session_id().as_deref(), Some(sid.as_str()));
        assert!(r#loop::is_binding_generation_current(gen));
        let events = r#loop::drain_lifecycle_events();
        assert!(
            events.iter().all(|e| e.event != "onUnbound"),
            "shell_close must not emit onUnbound: {events:?}"
        );

        // Explicit Reset still works after shell close (pre-Reset rules).
        r#loop::reset_binding().expect("explicit Reset after shell close");
        assert_query_unbound(&r#loop::query_binding());
        assert_eq!(live_session_id(), None);
    });
}

#[test]
fn t4_defensive_cut_is_not_ui_only_weak_path() {
    with_sandbox(|| {
        // Forbidden: "只清 UI" — must truly unbound + invalidate gen + cut session + cancel.
        let master = create_bound_plan("t4-no-weak");
        arm_plan_binding(&master);
        let _ = r#loop::open_ai_assistant_core(&master).unwrap();
        let gen = r#loop::query_binding().generation.expect("gen");
        r#loop::set_busy_for_tests(true);
        r#loop::clear_lifecycle_events_for_tests();

        r#loop::defensive_unbound().expect("full cut");

        assert_eq!(r#loop::binding_state(), "unbound");
        assert_eq!(r#loop::current_binding_slot_count(), 0);
        assert!(!r#loop::is_binding_generation_current(gen));
        assert_eq!(live_session_id(), None);
        assert!(r#loop::is_chat_cancelled_for_tests());
        assert_eq!(
            event_names(&r#loop::drain_lifecycle_events()),
            vec!["onUnbound"]
        );
        // Defensive cut does not replace/omit the need for business explicit Reset —
        // after cut, explicit Reset remains valid (idempotent) and is still the primary path.
        r#loop::reset_binding().expect("explicit Reset still the primary path");
        assert_eq!(r#loop::binding_state(), "unbound");
    });
}

// --- T5: shell sync (binding-changed) + distinguishable Host reject reasons ---

#[test]
fn t5_reset_cut_records_shell_binding_changed() {
    with_sandbox(|| {
        r#loop::set_binding(valid_binding()).expect("Set");
        r#loop::clear_shell_sync_events_for_tests();
        r#loop::reset_binding().expect("Reset");
        let syncs = r#loop::drain_shell_sync_events();
        assert!(
            syncs.iter().any(|e| {
                e.event == "ai-assistant:binding-changed" && e.state == "unbound"
            }),
            "Reset cut must record shell binding-changed → unbound: {syncs:?}"
        );
    });
}

#[test]
fn t5_defensive_unbound_records_shell_binding_changed() {
    with_sandbox(|| {
        r#loop::set_binding(valid_binding()).expect("Set");
        r#loop::clear_shell_sync_events_for_tests();
        r#loop::defensive_unbound().expect("defensive cut");
        let syncs = r#loop::drain_shell_sync_events();
        assert!(
            syncs.iter().any(|e| {
                e.event == "ai-assistant:binding-changed" && e.state == "unbound"
            }),
            "defensive cut must record shell binding-changed (must not bypass shell sync): {syncs:?}"
        );
    });
}

#[test]
fn t5_successful_set_records_shell_binding_changed_bound() {
    with_sandbox(|| {
        r#loop::clear_shell_sync_events_for_tests();
        r#loop::set_binding(valid_binding()).expect("Set");
        let syncs = r#loop::drain_shell_sync_events();
        assert!(
            syncs.iter().any(|e| {
                e.event == "ai-assistant:binding-changed" && e.state == "bound"
            }),
            "successful Set must record shell binding-changed → bound: {syncs:?}"
        );
    });
}

#[test]
fn t5_replace_set_records_shell_binding_changed_for_new_bound() {
    with_sandbox(|| {
        r#loop::set_binding(binding_with_prompt("v1")).expect("first Set");
        r#loop::clear_shell_sync_events_for_tests();
        r#loop::set_binding(binding_with_prompt("v2")).expect("replace Set");
        let syncs = r#loop::drain_shell_sync_events();
        assert!(
            syncs.iter().any(|e| {
                e.event == "ai-assistant:binding-changed" && e.state == "bound"
            }),
            "replace Set (new Bound) must record shell binding-changed: {syncs:?}"
        );
    });
}

#[test]
fn t5_shell_close_does_not_record_shell_binding_changed() {
    with_sandbox(|| {
        r#loop::set_binding(valid_binding()).expect("Set");
        r#loop::clear_shell_sync_events_for_tests();
        r#loop::shell_close_core().expect("关壳");
        let syncs = r#loop::drain_shell_sync_events();
        assert!(
            syncs
                .iter()
                .all(|e| e.event != "ai-assistant:binding-changed"),
            "关壳 ≠ 切断: must not emit shell binding-changed: {syncs:?}"
        );
    });
}

#[test]
fn t5_idempotent_unbound_cut_does_not_repeat_shell_binding_changed() {
    with_sandbox(|| {
        r#loop::set_binding(valid_binding()).expect("Set");
        r#loop::reset_binding().expect("Reset");
        r#loop::clear_shell_sync_events_for_tests();
        r#loop::defensive_unbound().expect("idempotent defensive");
        let syncs = r#loop::drain_shell_sync_events();
        assert!(
            syncs
                .iter()
                .all(|e| e.event != "ai-assistant:binding-changed"),
            "idempotent already-unbound cut must not re-emit shell binding-changed: {syncs:?}"
        );
    });
}

#[test]
fn t5_host_reject_reasons_are_distinguishable() {
    with_sandbox(|| {
        // 1) unbound
        let unbound_err = r#loop::execute_binding().expect_err("unbound");
        assert_eq!(unbound_err.as_code(), "rejected_unbound");

        // 2) in-flight cancel (reset_cancelled)
        r#loop::set_binding(valid_binding()).expect("Set");
        let cancel_err = r#loop::execute_binding_during(|| {
            r#loop::reset_binding().expect("Reset mid-execute");
        })
        .expect_err("cancelled");
        assert_eq!(cancel_err.as_code(), "reset_cancelled");

        // 3) stale generation
        r#loop::set_binding(valid_binding()).expect("Set again");
        let stale_err = r#loop::execute_binding_during(|| {
            r#loop::set_binding(binding_with_prompt("replaced")).expect("replace");
        })
        .expect_err("stale");
        assert_eq!(stale_err.as_code(), "rejected_stale_generation");

        // 4) non-live session — distinguishable code on return body / signal
        let master = create_bound_plan("t5-reject-codes");
        arm_plan_binding(&master);
        let open = r#loop::open_ai_assistant_core(&master).unwrap();
        let live = open["session_id"].as_str().unwrap().to_string();
        let foreign = session::create_session(None, None).unwrap().session_id;
        assert_ne!(foreign, live);
        let rejected = r#loop::agent_chat_turn_core(&foreign, "ping", Some(&master))
            .expect("chat returns body with reject signal");
        let not_live_code = rejected.body["code"]
            .as_str()
            .expect("non-live reject must expose distinguishable code");
        assert_eq!(not_live_code, "rejected_not_live_session");

        // All four Host reject signals must be pairwise distinct.
        let codes = [
            unbound_err.as_code(),
            cancel_err.as_code(),
            stale_err.as_code(),
            not_live_code,
        ];
        for i in 0..codes.len() {
            for j in (i + 1)..codes.len() {
                assert_ne!(
                    codes[i], codes[j],
                    "Host reject reasons must be distinguishable: {codes:?}"
                );
            }
        }
    });
}

#[test]
fn t5_not_live_session_reject_code_survives_after_cut() {
    with_sandbox(|| {
        let master = create_bound_plan("t5-cut-session-code");
        arm_plan_binding(&master);
        let open = r#loop::open_ai_assistant_core(&master).unwrap();
        let old_sid = open["session_id"].as_str().unwrap().to_string();
        r#loop::reset_binding().expect("cut");
        let rejected = r#loop::agent_chat_turn_core(&old_sid, "after-cut", Some(&master))
            .expect("reject body");
        assert_eq!(rejected.body["code"], "rejected_not_live_session");
        assert_ne!(rejected.body["terminal"], "none");
        assert_eq!(rejected.body["wrote"], false);
    });
}


// --- T6: Todos leave still explicit Reset + layered acceptance L0/L1/L2 ---
// shell_close is NOT cut acceptance. Business ids must not enter Binding Contract.

#[test]
fn t6_layered_acceptance_markers_are_landed() {
    assert_eq!(
        r#loop::LAYERED_ACCEPTANCE_L0,
        [
            "unbound_reject_execute",
            "reset_idempotent",
            "mid_reset_cancel",
        ]
    );
    assert_eq!(
        r#loop::LAYERED_ACCEPTANCE_L1,
        [
            "old_session_not_executable_after_reset_or_replace_set",
            "stale_generation_reject_continue",
            "re_set_without_old_turns",
            "any_successful_set_clears_pre_set_session",
        ]
    );
    assert_eq!(
        r#loop::LAYERED_ACCEPTANCE_L2,
        ["missed_dispose_or_reset_defensive_cut_not_executable"]
    );
    // Explicit leave→Reset remains primary; defensive cut does not replace it.
    assert_eq!(
        r#loop::TODOS_EXPLICIT_LEAVE_RESET_PRIMARY,
        r#loop::DEFENSIVE_CUT_EXPLICIT_RESET_CHAIN
    );
}

#[test]
fn t6_l0_unbound_reject_reset_idempotent_and_mid_reset_cancel() {
    with_sandbox(|| {
        // unbound reject
        let unbound_err = r#loop::execute_binding().expect_err("unbound must reject");
        assert_eq!(unbound_err.as_code(), "rejected_unbound");

        // Reset idempotent
        let master = create_bound_plan("t6-l0-idempotent");
        arm_plan_binding(&master);
        r#loop::reset_binding().expect("Reset");
        r#loop::reset_binding().expect("idempotent Reset");
        assert_eq!(r#loop::binding_state(), "unbound");
        let after = r#loop::execute_binding().expect_err("still unbound");
        assert_eq!(after.as_code(), "rejected_unbound");

        // mid-Reset cancel
        arm_plan_binding(&master);
        let cancel_err = r#loop::execute_binding_during(|| {
            r#loop::reset_binding().expect("Reset mid-execute");
        })
        .expect_err("mid-Reset must cancel execute");
        assert_eq!(cancel_err.as_code(), "reset_cancelled");
        assert_eq!(r#loop::binding_state(), "unbound");
    });
}

#[test]
fn t6_l1_session_generation_and_re_set_cuts() {
    with_sandbox(|| {
        let master = create_bound_plan("t6-l1-session");
        arm_plan_binding(&master);
        let open = r#loop::open_ai_assistant_core(&master).unwrap();
        let old_sid = open["session_id"].as_str().unwrap().to_string();
        let old_gen = r#loop::query_binding().generation.expect("gen");

        // Reset → old session not executable
        r#loop::reset_binding().expect("Reset");
        let rejected = r#loop::agent_chat_turn_core(&old_sid, "stale", Some(&master))
            .expect("reject body");
        assert_eq!(rejected.body["code"], "rejected_not_live_session");
        assert!(
            !r#loop::is_binding_generation_current(old_gen),
            "generation must be invalid after Reset"
        );

        // Any successful Set clears pre-set session
        let pre = r#loop::ensure_chat_session_core().expect("pre-set session");
        let pre_sid = pre["session_id"].as_str().unwrap().to_string();
        let master2 = create_bound_plan("t6-l1-reset2");
        arm_plan_binding(&master2);
        assert_eq!(
            live_session_id(),
            None,
            "successful Set must clear pre-set current_session_id"
        );
        assert_ne!(
            live_session_id().as_deref(),
            Some(pre_sid.as_str()),
            "pre-set session must not remain live after Set"
        );

        let open2 = r#loop::open_ai_assistant_core(&master2).unwrap();
        let sid_a = open2["session_id"].as_str().unwrap().to_string();
        // replace Set → re-Set without old session driving executable chat
        let master3 = create_bound_plan("t6-l1-replace");
        arm_plan_binding(&master3);
        assert_eq!(live_session_id(), None);
        let open3 = r#loop::open_ai_assistant_core(&master3).unwrap();
        let sid_b = open3["session_id"].as_str().unwrap().to_string();
        assert_ne!(sid_a, sid_b, "re-Set must mint a new session");
        let old_chat = r#loop::agent_chat_turn_core(&sid_a, "old-ctx", Some(&master3))
            .expect("reject old");
        assert_eq!(old_chat.body["code"], "rejected_not_live_session");
    });
}

#[test]
fn t6_l1_stale_generation_rejects_continue() {
    with_sandbox(|| {
        let master = create_bound_plan("t6-l1-gen");
        arm_plan_binding(&master);
        let err = r#loop::execute_binding_during(|| {
            let other = create_bound_plan("t6-l1-gen-b");
            arm_plan_binding(&other);
        })
        .expect_err("stale generation must reject continue");
        assert_eq!(err.as_code(), "rejected_stale_generation");
    });
}

#[test]
fn t6_l2_missed_reset_defensive_cut_then_not_executable() {
    with_sandbox(|| {
        let master = create_bound_plan("t6-l2-missed");
        arm_plan_binding(&master);
        let open = r#loop::open_ai_assistant_core(&master).unwrap();
        let old_sid = open["session_id"].as_str().unwrap().to_string();
        // Missed dispose/Reset: Host defensive cut backs the leave signal.
        r#loop::defensive_unbound().expect("defensive cut");
        assert_eq!(r#loop::binding_state(), "unbound");
        let exec = r#loop::execute_binding().expect_err("not executable after L2 cut");
        assert_eq!(exec.as_code(), "rejected_unbound");
        let chat = r#loop::agent_chat_turn_core(&old_sid, "after-miss", Some(&master))
            .expect("reject");
        assert_eq!(chat.body["code"], "rejected_not_live_session");
    });
}

#[test]
fn t6_shell_close_is_not_cut_acceptance() {
    with_sandbox(|| {
        let master = create_bound_plan("t6-shell-close");
        arm_plan_binding(&master);
        let gen = r#loop::query_binding().generation.expect("gen");
        r#loop::shell_close_core().expect("shell close");
        // 关壳 ≠ 切断：仍 bound / generation current / executable
        assert_eq!(r#loop::binding_state(), "bound");
        assert!(r#loop::is_binding_generation_current(gen));
        r#loop::execute_binding().expect("shell_close must not cut execute");
        // Explicit Reset remains the primary leave path
        r#loop::reset_binding().expect("explicit Reset");
        assert_eq!(r#loop::binding_state(), "unbound");
    });
}

#[test]
fn t6_explicit_reset_not_omitted_because_defensive_exists() {
    with_sandbox(|| {
        // Defensive cut exists, but normal leave still uses explicit Reset semantics.
        assert_eq!(
            r#loop::TODOS_EXPLICIT_LEAVE_RESET_PRIMARY.last().copied(),
            Some("src-tauri/src/services/agent/loop.rs::reset_binding")
        );
        let master = create_bound_plan("t6-explicit-primary");
        arm_plan_binding(&master);
        r#loop::clear_lifecycle_events_for_tests();
        r#loop::reset_binding().expect("explicit Reset primary path");
        let events = r#loop::drain_lifecycle_events();
        assert!(
            events.iter().any(|e| e.event == "onUnbound"),
            "explicit Reset must emit onUnbound: {events:?}"
        );
        // Defensive after explicit is idempotent — does not replace leave duty.
        r#loop::defensive_unbound().expect("idempotent defensive");
        assert_eq!(r#loop::binding_state(), "unbound");
    });
}

#[test]
fn t6_binding_contract_rejects_top_level_business_ids() {
    with_sandbox(|| {
        // Business ids must not appear on Binding Contract query surface.
        r#loop::set_binding(valid_binding()).expect("Set without business top-level");
        let q = r#loop::query_binding();
        assert_query_is_business_agnostic(&q);
    });
}

// --- t2: key-only Binding Set/Reset + MCP session capability context ---

fn key_only_payload(key: &str) -> Value {
    json!({ "key": key })
}

#[test]
fn t2_key_only_set_loads_mcp_server_into_session_capability_context() {
    with_sandbox(|| {
        use crate::services::mcp_server_registry::{self, SEEDED_BUSINESS_KEY};
        assert_eq!(r#loop::binding_state(), "unbound");
        assert!(r#loop::loaded_mcp_server().is_none());

        r#loop::try_set_binding_json(&key_only_payload(SEEDED_BUSINESS_KEY))
            .expect("legal key Set must succeed");
        assert_eq!(r#loop::binding_state(), "bound");

        let loaded = r#loop::loaded_mcp_server().expect("Set must load MCP Server config");
        let expected = mcp_server_registry::lookup(SEEDED_BUSINESS_KEY).expect("registry");
        assert_eq!(loaded, expected);
        assert!(
            !loaded.capability_description.trim().is_empty(),
            "loaded config must be decision-level non-empty"
        );
    });
}

#[test]
fn t2_reset_binding_unloads_mcp_server_config() {
    with_sandbox(|| {
        use crate::services::mcp_server_registry::SEEDED_BUSINESS_KEY;
        r#loop::try_set_binding_json(&key_only_payload(SEEDED_BUSINESS_KEY)).expect("Set");
        assert!(r#loop::loaded_mcp_server().is_some());

        r#loop::reset_binding().expect("Reset");
        assert_eq!(r#loop::binding_state(), "unbound");
        assert!(
            r#loop::loaded_mcp_server().is_none(),
            "Reset must unload MCP Server config from session capability context"
        );
    });
}

#[test]
fn t2_set_reset_public_json_reject_engine_selection_params() {
    with_sandbox(|| {
        use crate::services::mcp_server_registry::SEEDED_BUSINESS_KEY;
        // Public Set must not accept engine selection parameters.
        for payload in [
            json!({ "key": SEEDED_BUSINESS_KEY, "engine": "host" }),
            json!({ "key": SEEDED_BUSINESS_KEY, "engine_type": "cursor" }),
            json!({ "key": SEEDED_BUSINESS_KEY, "engineType": "host" }),
        ] {
            let err = r#loop::try_set_binding_json(&payload)
                .expect_err("engine selection params must fail at public boundary");
            assert_eq!(err.as_code(), "set_invalid");
            assert_eq!(r#loop::binding_state(), "unbound");
            assert!(r#loop::loaded_mcp_server().is_none());
        }
        // reset_binding takes no engine params (signature-level); idempotent ok.
        r#loop::reset_binding().expect("Reset");
    });
}

#[test]
fn t2_replace_set_with_new_key_replaces_loaded_mcp_config() {
    with_sandbox(|| {
        use crate::services::mcp_server_registry::{self, McpServerConfig, SEEDED_BUSINESS_KEY};
        r#loop::try_set_binding_json(&key_only_payload(SEEDED_BUSINESS_KEY)).expect("first Set");
        let first = r#loop::loaded_mcp_server().expect("first loaded");

        mcp_server_registry::register(
            "alt_business_key",
            McpServerConfig {
                capability_description: "alternate mcp capability".into(),
                http_transport: mcp_server_registry::HttpMcpTransport {
                    name: "workbench".into(),
                    url: "http://127.0.0.1:9876/mcp".into(),
                    headers: Default::default(),
                },
            },
        )
        .expect("register alt");
        r#loop::try_set_binding_json(&key_only_payload("alt_business_key")).expect("replace Set");

        let second = r#loop::loaded_mcp_server().expect("replaced loaded");
        assert_ne!(first, second, "replace must not keep old MCP config alongside new");
        assert_eq!(second.capability_description, "alternate mcp capability");
        assert_eq!(r#loop::binding_state(), "bound");
        assert_eq!(r#loop::current_binding_slot_count(), 1);
    });
}

#[test]
fn t2_reset_when_unbound_is_idempotent_mcp_slot_stays_empty() {
    with_sandbox(|| {
        assert!(r#loop::loaded_mcp_server().is_none());
        r#loop::reset_binding().expect("Reset unbound");
        assert_eq!(r#loop::binding_state(), "unbound");
        assert!(r#loop::loaded_mcp_server().is_none());
        r#loop::reset_binding().expect("second Reset unbound");
        assert!(r#loop::loaded_mcp_server().is_none());
    });
}

#[test]
fn t2_unknown_key_fails_explicitly_without_destroying_prior_mcp_context() {
    with_sandbox(|| {
        use crate::services::mcp_server_registry::SEEDED_BUSINESS_KEY;
        r#loop::try_set_binding_json(&key_only_payload(SEEDED_BUSINESS_KEY)).expect("seed bound");
        let prior = r#loop::loaded_mcp_server().expect("prior MCP");
        let gen = r#loop::query_binding().generation;

        let err = r#loop::try_set_binding_json(&key_only_payload("unknown_business_key_xyz"))
            .expect_err("unknown key must fail");
        assert_eq!(err.as_code(), "unknown_key");
        assert_eq!(r#loop::binding_state(), "bound");
        assert_eq!(
            r#loop::loaded_mcp_server().as_ref(),
            Some(&prior),
            "failed Set must not destroy prior session capability context"
        );
        assert_eq!(r#loop::query_binding().generation, gen);
    });
}

#[test]
fn t2_legacy_tools_prompt_callbacks_payload_rejected_at_public_boundary() {
    with_sandbox(|| {
        use crate::services::mcp_server_registry::SEEDED_BUSINESS_KEY;
        // Unbound: legacy payload cannot bypass key→MCP lookup.
        let err = r#loop::try_set_binding_json(&json!({
            "tools": [{ "name": "tool_a", "handle": "opaque-tool-a" }],
            "prompt": "opaque-system-prompt",
            "callbacks": {}
        }))
        .expect_err("legacy payload must fail");
        assert_eq!(err.as_code(), "set_invalid");
        assert_eq!(r#loop::binding_state(), "unbound");
        assert!(r#loop::loaded_mcp_server().is_none());

        // Bound via key: legacy payload still rejected; MCP context preserved.
        r#loop::try_set_binding_json(&key_only_payload(SEEDED_BUSINESS_KEY)).expect("key Set");
        let prior = r#loop::loaded_mcp_server().expect("loaded");
        let err = r#loop::try_set_binding_json(&json!({
            "tools": [{ "name": "tool_a" }],
            "prompt": "p",
            "callbacks": {}
        }))
        .expect_err("legacy payload must fail when bound");
        assert_eq!(err.as_code(), "set_invalid");
        assert_eq!(r#loop::binding_state(), "bound");
        assert_eq!(r#loop::loaded_mcp_server().as_ref(), Some(&prior));
    });
}

#[test]
fn t2_missing_or_invalid_key_input_fails_explicitly() {
    with_sandbox(|| {
        for payload in [
            json!({}),
            json!({ "key": "" }),
            json!({ "key": "   " }),
            json!({ "key": null }),
            json!({ "master_task_id": "task_x" }),
            json!("todo_task"),
        ] {
            let err = r#loop::try_set_binding_json(&payload)
                .expect_err("missing/invalid key must fail");
            assert_eq!(err.as_code(), "set_invalid", "payload={payload}");
            assert_eq!(r#loop::binding_state(), "unbound");
            assert!(r#loop::loaded_mcp_server().is_none());
        }
    });
}

#[test]
fn t2_binding_from_json_accepts_key_only_rejects_legacy() {
    with_sandbox(|| {
        use crate::services::agent::session;
        use crate::services::mcp_server_registry::SEEDED_BUSINESS_KEY;
        let b = session::binding_from_json(&key_only_payload(SEEDED_BUSINESS_KEY))
            .expect("key-only parse");
        assert_eq!(
            session::binding_business_key(&b).as_deref(),
            Some(SEEDED_BUSINESS_KEY),
            "parsed Binding must carry the business key"
        );

        let err = session::binding_from_json(&json!({
            "tools": [{ "name": "t" }],
            "prompt": "p",
            "callbacks": {}
        }))
        .expect_err("legacy payload");
        assert_eq!(err.as_code(), "set_invalid");
    });
}

// --- t4: session capability context read-only consumption face ---
//
// Locks the engine-facing read API for future Host Loop + Cursor Local adapters.
// No engine-branch injection code in this slice (L3).
//
// A1 confirmed (not narrowed): the same decision-level McpServerConfig shape
// returned by this read face is the shared consumption form for both engines;
// field-level transport schema (stdio/http/…) remains deferred.
//
// A2 confirmed (not narrowed): Host mcp_server_registry is the sole lookup
// source; this face only exposes config already loaded by key-only Set from
// that authoritative table — it does not re-resolve or accept legacy payloads.

#[test]
fn t4_read_face_exposes_decision_level_shape_matching_registry_value() {
    with_sandbox(|| {
        use crate::services::mcp_server_registry::{self, SEEDED_BUSINESS_KEY};
        r#loop::try_set_binding_json(&key_only_payload(SEEDED_BUSINESS_KEY)).expect("Set");

        let view = r#loop::session_capability_mcp_config()
            .expect("bound session must expose loaded MCP config via read face");
        let expected = mcp_server_registry::lookup(SEEDED_BUSINESS_KEY).expect("registry");

        // Decision-level shape parity with t1 value (capability_description only).
        assert_eq!(view, expected);
        assert!(
            !view.capability_description.trim().is_empty(),
            "read face must expose non-empty decision-level capability description"
        );
        // Shared form for both future engines (A1): same type/shape, no engine param.
        assert_eq!(
            view.capability_description,
            expected.capability_description,
            "Host Loop and Cursor Local must consume the same decision-level fields"
        );
    });
}

#[test]
fn t4_read_face_returns_none_when_unbound_or_after_reset() {
    with_sandbox(|| {
        use crate::services::mcp_server_registry::SEEDED_BUSINESS_KEY;
        assert!(
            r#loop::session_capability_mcp_config().is_none(),
            "unbound consumption face must be empty/None"
        );

        r#loop::try_set_binding_json(&key_only_payload(SEEDED_BUSINESS_KEY)).expect("Set");
        assert!(r#loop::session_capability_mcp_config().is_some());

        r#loop::reset_binding().expect("Reset");
        assert!(
            r#loop::session_capability_mcp_config().is_none(),
            "Reset must unload; read face must report removed"
        );
    });
}

#[test]
fn t4_read_face_is_readonly_consumer_mutate_does_not_rewrite_session() {
    with_sandbox(|| {
        use crate::services::mcp_server_registry::SEEDED_BUSINESS_KEY;
        r#loop::try_set_binding_json(&key_only_payload(SEEDED_BUSINESS_KEY)).expect("Set");
        let original = r#loop::session_capability_mcp_config().expect("loaded");

        // Consumer holds a detached view; mutating it must not rewrite session state.
        let mut local = r#loop::session_capability_mcp_config().expect("clone view");
        local.capability_description = "mutated-by-consumer".into();
        assert_ne!(local.capability_description, original.capability_description);

        let reread = r#loop::session_capability_mcp_config().expect("still loaded");
        assert_eq!(
            reread, original,
            "read face must not provide a business/external write path into session config"
        );
    });
}

#[test]
fn t4_only_set_reset_lifecycle_may_change_loaded_config() {
    with_sandbox(|| {
        use crate::services::mcp_server_registry::{self, McpServerConfig, SEEDED_BUSINESS_KEY};
        assert!(r#loop::session_capability_mcp_config().is_none());

        // Lifecycle Set loads.
        r#loop::try_set_binding_json(&key_only_payload(SEEDED_BUSINESS_KEY)).expect("Set");
        let first = r#loop::session_capability_mcp_config().expect("after Set");

        // Registry mutation alone must not rewrite the already-loaded session view
        // (A2: table is lookup source at Set time; read face does not re-inject).
        mcp_server_registry::register(
            SEEDED_BUSINESS_KEY,
            McpServerConfig {
                capability_description: "registry-mutated-after-set".into(),
                http_transport: mcp_server_registry::HttpMcpTransport {
                    name: "workbench".into(),
                    url: "http://127.0.0.1:9876/mcp".into(),
                    headers: Default::default(),
                },
            },
        )
        .expect("register overwrite");
        let still = r#loop::session_capability_mcp_config().expect("unchanged without Set");
        assert_eq!(
            still, first,
            "read face must not re-lookup/re-inject; only Set/Reset lifecycle may change"
        );

        // Lifecycle replace Set updates.
        mcp_server_registry::register(
            "alt_for_t4",
            McpServerConfig {
                capability_description: "alt capability for t4".into(),
                http_transport: mcp_server_registry::HttpMcpTransport {
                    name: "workbench".into(),
                    url: "http://127.0.0.1:9876/mcp".into(),
                    headers: Default::default(),
                },
            },
        )
        .expect("alt");
        r#loop::try_set_binding_json(&key_only_payload("alt_for_t4")).expect("replace Set");
        let second = r#loop::session_capability_mcp_config().expect("after replace");
        assert_ne!(second, first);
        assert_eq!(second.capability_description, "alt capability for t4");

        // Lifecycle Reset clears.
        r#loop::reset_binding().expect("Reset");
        assert!(r#loop::session_capability_mcp_config().is_none());
    });
}

#[test]
fn t4_read_face_cannot_reinject_legacy_tools_prompt_callbacks() {
    with_sandbox(|| {
        use crate::services::mcp_server_registry::SEEDED_BUSINESS_KEY;
        r#loop::try_set_binding_json(&key_only_payload(SEEDED_BUSINESS_KEY)).expect("Set");
        let via_read = r#loop::session_capability_mcp_config().expect("read face");

        // Consumption face returns decision-level MCP config only — not a Binding
        // write path. Legacy tools/prompt/callbacks cannot be pushed back through it.
        assert!(
            !via_read.capability_description.contains("\"tools\""),
            "read face must not surface legacy tools payload shape"
        );
        // Public Set still rejects legacy payload; session view unchanged.
        let err = r#loop::try_set_binding_json(&json!({
            "tools": [{ "name": "tool_a", "handle": "opaque" }],
            "prompt": via_read.capability_description,
            "callbacks": {}
        }))
        .expect_err("legacy reinject via Set must fail");
        assert_eq!(err.as_code(), "set_invalid");
        assert_eq!(
            r#loop::session_capability_mcp_config().as_ref(),
            Some(&via_read),
            "failed legacy reinject must leave read-face config unchanged"
        );
    });
}

#[test]
fn t4_a1_a2_handoff_assumptions_confirmed_not_narrowed() {
    with_sandbox(|| {
        use crate::services::mcp_server_registry::{self, SEEDED_BUSINESS_KEY};
        // A1: one decision-level shape, dual-engine readable (no engine-specific fields).
        r#loop::try_set_binding_json(&key_only_payload(SEEDED_BUSINESS_KEY)).expect("Set");
        let face = r#loop::session_capability_mcp_config().expect("face");
        let table = mcp_server_registry::lookup(SEEDED_BUSINESS_KEY).expect("table");
        assert_eq!(
            face, table,
            "A1 confirmed: read face exposes the same decision-level form both engines will read"
        );

        // A2: Host authoritative table is the sole lookup source at Set; Binding
        // callers only need the stable business key (already exercised by Set path).
        assert_eq!(
            face.capability_description, table.capability_description,
            "A2 confirmed: loaded view originates from Host registry lookup, not caller payload"
        );
        assert!(
            r#loop::SESSION_CAPABILITY_READ_FACE_A1_DUAL_ENGINE_SAME_SHAPE,
            "A1 must be explicitly confirmed in delivery (not silently narrowed)"
        );
        assert!(
            r#loop::SESSION_CAPABILITY_READ_FACE_A2_HOST_REGISTRY_SOLE_LOOKUP,
            "A2 must be explicitly confirmed in delivery (not silently narrowed)"
        );
    });
}

// --- T3 / P3: Host Loop empty tools + zero Cursor ---

#[test]
fn t3_host_business_chat_sends_empty_tools_to_llm() {
    with_sandbox(|| {
        let master = create_bound_plan("t3-empty-tools");
        let mock = spawn_scripted_llm(vec![assistant_text("门面会话可用")]);
        install_llm_cfg(&mock);
        arm_plan_binding(&master);
        let open = r#loop::open_ai_assistant_core(&master).unwrap();
        let sid = open["session_id"].as_str().unwrap();
        let result = r#loop::agent_chat_turn_core(sid, "你好", Some(&master)).unwrap();
        assert_eq!(result.body["terminal"], "none");
        assert_eq!(result.body["wrote"], false);
        let hits = mock.hits.lock().unwrap();
        assert_eq!(hits.len(), 1);
        assert_host_llm_tools_empty(&hits[0]);
    });
}

#[test]
fn t3_host_facade_open_ensure_chat_works_without_cursor() {
    with_sandbox(|| {
        let master = create_bound_plan("t3-no-cursor");
        let mock = spawn_scripted_llm(vec![assistant_text("无 Cursor 亦可完成门面会话")]);
        install_llm_cfg(&mock);
        arm_plan_binding(&master);

        let open = r#loop::open_ai_assistant_core(&master).unwrap();
        let sid = open["session_id"].as_str().unwrap().to_string();
        assert!(!sid.is_empty());

        let ensure = r#loop::ensure_chat_session_core().unwrap();
        assert_eq!(ensure["session_id"], sid);

        let result = r#loop::agent_chat_turn_core(&sid, "继续", Some(&master)).unwrap();
        assert_eq!(result.body["terminal"], "none");
        assert_eq!(result.body["wrote"], false);
        assert!(result.emit_turn_completed.is_some());
        assert!(
            result.body["reply_text"]
                .as_str()
                .unwrap_or("")
                .contains("门面会话")
        );
        assert_host_llm_tools_empty(&mock.hits.lock().unwrap()[0]);
    });
}

#[test]
fn t3_host_path_modules_do_not_import_cursor_adapter() {
    // Module boundary: Host loop/llm must not import Cursor SDK / cursor_adapter.
    let loop_src = include_str!("../../../services/agent/loop.rs");
    let llm_src = include_str!("../../../services/agent/llm.rs");
    for (label, src) in [("loop.rs", loop_src), ("llm.rs", llm_src)] {
        let lower = src.to_ascii_lowercase();
        assert!(
            !lower.contains("cursor_adapter"),
            "{label} must not import/link cursor_adapter"
        );
        assert!(
            !lower.contains("cursor_sdk"),
            "{label} must not import/link Cursor SDK"
        );
        assert!(
            !src.contains("cursor_agent") && !src.contains("CursorAgent"),
            "{label} must not reference Cursor Agent SDK symbols"
        );
    }
}

#[test]
fn t3_host_reads_session_capability_mcp_config_readonly_without_tool_dispatch() {
    with_sandbox(|| {
        use crate::services::mcp_server_registry::SEEDED_BUSINESS_KEY;
        let mock = spawn_scripted_llm(vec![assistant_text("读只读面后文本回复")]);
        install_llm_cfg(&mock);
        r#loop::try_set_binding_json(&key_only_payload(SEEDED_BUSINESS_KEY)).expect("Set");
        let before = r#loop::session_capability_mcp_config().expect("loaded");

        // Host adapter may consume the L2 read face; must not mutate it via chat.
        let open = r#loop::ensure_chat_session_core().unwrap();
        let sid = open["session_id"].as_str().unwrap();
        let result = r#loop::agent_chat_turn_core(sid, "ping", None).unwrap();
        assert_eq!(result.body["terminal"], "none");
        assert_eq!(result.body["wrote"], false);

        let after = r#loop::session_capability_mcp_config().expect("still loaded");
        assert_eq!(after, before, "chat must not rewrite session capability MCP config");
        assert_host_llm_tools_empty(&mock.hits.lock().unwrap()[0]);
    });
}

#[test]
fn t3_host_unexpected_tool_calls_never_call_process_dispatch() {
    with_sandbox(|| {
        let master = create_bound_plan("t3-no-dispatch");
        let title_before = todo_task::get_by_id(&master)["title"].clone();
        let mock = spawn_scripted_llm(vec![assistant_tools(
            json!([{
                "id": "t3nd1",
                "type": "function",
                "function": {
                    "name": "update_master_title",
                    "arguments": "{\"title\":\"must-not-write\"}"
                }
            }]),
            None,
        )]);
        install_llm_cfg(&mock);
        arm_plan_binding(&master);
        let open = r#loop::open_ai_assistant_core(&master).unwrap();
        let sid = open["session_id"].as_str().unwrap();
        let result = r#loop::agent_chat_turn_core(sid, "改标题", Some(&master)).unwrap();
        assert_eq!(result.body["wrote"], false);
        assert_eq!(result.body["terminal"], "error");
        assert_eq!(todo_task::get_by_id(&master)["title"], title_before);
        let sess = session::load_session(sid).unwrap();
        assert!(sess.turns.iter().all(|t| t.role != "tool"));
        assert_host_llm_tools_empty(&mock.hits.lock().unwrap()[0]);
    });
}

#[test]
fn t3_a3_host_empty_tools_facade_usable_confirmed() {
    // Must Close Before F-45 / AC2: A3 confirmed (not silently narrowed).
    assert!(
        r#loop::HOST_EMPTY_TOOLS_A3_FACADE_USABLE,
        "A3 must be explicitly confirmed: Host empty tools still usable for facade session"
    );
    with_sandbox(|| {
        let master = create_bound_plan("t3-a3");
        let mock = spawn_scripted_llm(vec![assistant_text("A3: empty tools facade ok")]);
        install_llm_cfg(&mock);
        arm_plan_binding(&master);
        let open = r#loop::open_ai_assistant_core(&master).unwrap();
        let sid = open["session_id"].as_str().unwrap();
        let result = r#loop::agent_chat_turn_core(sid, "hi", Some(&master)).unwrap();
        assert_eq!(result.body["terminal"], "none");
        assert_eq!(result.body["wrote"], false);
        assert_host_llm_tools_empty(&mock.hits.lock().unwrap()[0]);
    });
}

// ── L1+L2 integration retained on L3 Host empty-tools path ──

#[test]
fn t3_loop_rs_has_no_business_whitelist_dispatch_path() {
    let src = include_str!("../../../services/agent/loop.rs");
    // Comments may mention the ban; forbid call/import forms only.
    assert!(
        !src.contains("tools::dispatch(") && !src.contains("use crate::services::agent::tools"),
        "loop.rs must not call/import tools::dispatch"
    );
    assert!(
        !src.contains("const WHITELIST"),
        "loop.rs must not keep business WHITELIST for in-process tool-round"
    );
    assert!(
        !src.contains("const WRITE_TOOLS"),
        "loop.rs must not keep WRITE_TOOLS tied to in-process dispatch"
    );
}

#[test]
fn t3_nonempty_tools_binding_tool_calls_never_mutate_todo_task() {
    // Even with non-empty Binding.tools, Host L3 path never dispatches in-process.
    with_sandbox(|| {
        let master = create_bound_plan("t3非空tools");
        let before = todo_task::get_by_id(&master)["title"].clone();
        let mock = spawn_scripted_llm(vec![assistant_tools(
            json!([{
                "id": "t3_dispatch_gone",
                "type": "function",
                "function": {
                    "name": "update_master_title",
                    "arguments": "{\"title\":\"不得进程内写入\"}"
                }
            }]),
            None,
        )]);
        let mut sess = session::create_session(Some(&master), Some("t3非空tools")).unwrap();
        arm_plan_binding(&master);
        let out = r#loop::run_loop(&mut sess, "改标题", &cfg_for(&mock));
        assert_eq!(out.wrote, false);
        assert_eq!(todo_task::get_by_id(&master)["title"], before);
        assert!(
            !sess.turns.iter().any(|t| t.role == "tool"),
            "no tool-role turns from removed dispatch path"
        );
        assert_eq!(out.terminal, Terminal::Error);
        assert_host_llm_tools_empty(&mock.hits.lock().unwrap()[0]);
    });
}

fn empty_tools_binding() -> r#loop::Binding {
    r#loop::Binding {
        tools: json!([]),
        prompt: json!(PLAN_ASSISTANT_SYSTEM_PROMPT),
        callbacks: json!({}),
    }
}

#[test]
fn t2_empty_tools_binding_text_only_round_succeeds() {
    with_sandbox(|| {
        let master = create_bound_plan("空tools纯文本");
        let mock = spawn_scripted_llm(vec![assistant_text("纯文本回复，无工具")]);
        let mut sess = session::create_session(Some(&master), Some("空tools纯文本")).unwrap();
        r#loop::set_binding(empty_tools_binding()).expect("empty tools Set");
        let out = r#loop::run_loop(&mut sess, "你好", &cfg_for(&mock));
        assert_outcome(&out, "none", false);
        assert!(out.reply_text.contains("纯文本"));
        assert_host_llm_tools_empty(&mock.hits.lock().unwrap()[0]);
    });
}

#[test]
fn t2_key_only_set_llm_round_omits_tools_and_loads_mcp() {
    with_sandbox(|| {
        use crate::services::mcp_server_registry::SEEDED_BUSINESS_KEY;
        let master = create_bound_plan("key-only空tools");
        let mock = spawn_scripted_llm(vec![assistant_text("key-only无工具回复")]);
        let mut sess = session::create_session(Some(&master), Some("key-only空tools")).unwrap();
        r#loop::try_set_binding_json(&key_only_payload(SEEDED_BUSINESS_KEY))
            .expect("key-only Set");
        assert!(r#loop::loaded_mcp_server().is_some());
        assert!(r#loop::session_capability_mcp_config().is_some());
        let before = todo_task::get_by_id(&master)["title"].clone();
        let out = r#loop::run_loop(&mut sess, "你好", &cfg_for(&mock));
        assert_outcome(&out, "none", false);
        assert_eq!(out.wrote, false);
        assert!(
            !sess.turns.iter().any(|t| t.role == "tool"),
            "key-only business path must not dispatch in-process tools"
        );
        assert_eq!(todo_task::get_by_id(&master)["title"], before);
        assert_host_llm_tools_empty(&mock.hits.lock().unwrap()[0]);
    });
}

