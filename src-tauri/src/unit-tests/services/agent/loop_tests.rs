//! Loop + Host open/chat-turn contract tests (t4).

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
            "llm": {
                "base_url": format!("http://127.0.0.1:{}", mock.port),
                "model": "test-model",
                "platform": "openai_compatible"
            }
        }),
    );
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
    });
}

#[test]
fn run_loop_tool_write_sets_wrote_true_and_persists() {
    with_sandbox(|| {
        let master = create_bound_plan("写前标题");
        let mock = spawn_scripted_llm(vec![
            assistant_tools(
                json!([{
                    "id": "call_1",
                    "type": "function",
                    "function": {
                        "name": "update_master_title",
                        "arguments": "{\"title\":\"写后标题\"}"
                    }
                }]),
                None,
            ),
            assistant_text("已把主标题改为「写后标题」"),
        ]);
        let mut sess = session::create_session(Some(&master), Some("写前标题")).unwrap();
        arm_plan_binding(&master);
        let out = r#loop::run_loop(&mut sess, "把主标题改成写后标题", &cfg_for(&mock));
        assert_outcome(&out, "none", true);
        let got = todo_task::get_by_id(&master);
        assert_eq!(got["title"], "写后标题");
        assert!(
            sess.turns.iter().any(|t| t.role == "tool"),
            "expected tool turn in session"
        );
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
fn parallel_tool_calls_run_serially_and_ok_false_does_not_abort() {
    with_sandbox(|| {
        let master = create_bound_plan("批处理");
        let mock = spawn_scripted_llm(vec![
            assistant_tools(
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
            ),
            assistant_text("第一个失败了，但已新增子计划"),
        ]);
        let mut sess = session::create_session(Some(&master), Some("批处理")).unwrap();
        arm_plan_binding(&master);
        let out = r#loop::run_loop(&mut sess, "改不存在的子项并加一个", &cfg_for(&mock));
        assert_outcome(&out, "none", true);
        let tool_turns: Vec<_> = sess.turns.iter().filter(|t| t.role == "tool").collect();
        assert_eq!(tool_turns.len(), 2);
        let t1: Value = serde_json::from_str(tool_turns[0].content.as_deref().unwrap()).unwrap();
        let t2: Value = serde_json::from_str(tool_turns[1].content.as_deref().unwrap()).unwrap();
        assert_eq!(t1["ok"], false);
        assert_eq!(t2["ok"], true);
        let listed = todo_task::get_by_id(&master);
        let titles: Vec<_> = listed["sub_tasks"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| s["title"].as_str().unwrap().to_string())
            .collect();
        assert!(titles.iter().any(|t| t == "批后子项"));
        assert_eq!(mock.hits.lock().unwrap().len(), 2, "batch then callback LLM");
    });
}

#[test]
fn same_message_tool_calls_plus_content_content_is_not_final_reply() {
    with_sandbox(|| {
        let master = create_bound_plan("同条");
        let mock = spawn_scripted_llm(vec![
            assistant_tools(
                json!([{
                    "id": "c1",
                    "type": "function",
                    "function": {
                        "name": "add_sub_task",
                        "arguments": "{\"title\":\"同条子项\"}"
                    }
                }]),
                Some("这段 content 不是终态"),
            ),
            assistant_text("已新增同条子项"),
        ]);
        let mut sess = session::create_session(Some(&master), Some("同条")).unwrap();
        arm_plan_binding(&master);
        let out = r#loop::run_loop(&mut sess, "加子项", &cfg_for(&mock));
        assert_outcome(&out, "none", true);
        assert_eq!(out.reply_text, "已新增同条子项");
        assert!(!out.reply_text.contains("不是终态"));
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
fn run_loop_add_sub_and_update_sub_title_paths_are_observable() {
    with_sandbox(|| {
        let master = create_bound_plan("子路径");
        let listed = todo_task::get_by_id(&master);
        let sub_id = listed["sub_tasks"][0]["sub_task_id"]
            .as_str()
            .unwrap()
            .to_string();

        let upd_args = format!(
            "{{\"sub_task_id\":\"{sub_id}\",\"title\":\"改后子标题\"}}"
        );
        let mock = spawn_scripted_llm(vec![
            assistant_tools(
                json!([{
                    "id": "c_add",
                    "type": "function",
                    "function": {
                        "name": "add_sub_task",
                        "arguments": "{\"title\":\"新观察子项\"}"
                    }
                }]),
                None,
            ),
            assistant_text("已新增子计划"),
            assistant_tools(
                json!([{
                    "id": "c_upd",
                    "type": "function",
                    "function": {
                        "name": "update_sub_title",
                        "arguments": upd_args
                    }
                }]),
                None,
            ),
            assistant_text("已改子标题"),
        ]);
        let mut sess = session::create_session(Some(&master), Some("子路径")).unwrap();
        arm_plan_binding(&master);

        let out_add = r#loop::run_loop(&mut sess, "加一个子计划叫新观察子项", &cfg_for(&mock));
        assert_outcome(&out_add, "none", true);
        let titles_after_add: Vec<_> = todo_task::get_by_id(&master)["sub_tasks"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| s["title"].as_str().unwrap().to_string())
            .collect();
        assert!(titles_after_add.iter().any(|t| t == "新观察子项"));

        let out_upd = r#loop::run_loop(&mut sess, "把原子项标题改成改后子标题", &cfg_for(&mock));
        assert_outcome(&out_upd, "none", true);
        let updated = todo_task::get_by_id(&master);
        let old = updated["sub_tasks"]
            .as_array()
            .unwrap()
            .iter()
            .find(|s| s["sub_task_id"] == sub_id)
            .unwrap();
        assert_eq!(old["title"], "改后子标题");
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
    with_sandbox(|| {
        let master = create_bound_plan("工具上限");
        let mut responses = Vec::new();
        for i in 0..9 {
            responses.push(assistant_tools(
                json!([{
                    "id": format!("c{i}"),
                    "type": "function",
                    "function": {
                        "name": "get_plan",
                        "arguments": "{}"
                    }
                }]),
                None,
            ));
        }
        let mock = spawn_scripted_llm(responses);
        let mut sess = session::create_session(Some(&master), Some("工具上限")).unwrap();
        arm_plan_binding(&master);
        let out = r#loop::run_loop(&mut sess, "一直读", &cfg_for(&mock));
        assert_outcome(&out, "error", false);
        let tool_rounds = sess.turns.iter().filter(|t| t.role == "tool").count();
        assert!(tool_rounds <= 8, "tool_rounds={tool_rounds}");
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
        arm_plan_binding(&b);
        let second = r#loop::open_ai_assistant_core(&b).unwrap();
        assert_eq!(second["busy"], true);
        assert_eq!(second["session_id"], sid);
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
fn in_flight_writes_use_binding_at_turn_start_despite_busy_open() {
    with_sandbox(|| {
        let a = create_bound_plan("旧绑定");
        let b = create_bound_plan("新绑定");
        let mock = spawn_scripted_llm(vec![
            assistant_tools(
                json!([{
                    "id": "c1",
                    "type": "function",
                    "function": {
                        "name": "update_master_title",
                        "arguments": "{\"title\":\"落在旧绑定\"}"
                    }
                }]),
                None,
            ),
            assistant_text("已改旧绑定标题"),
        ]);
        install_llm_cfg(&mock);
        arm_plan_binding(&a);
        let open = r#loop::open_ai_assistant_core(&a).unwrap();
        let sid = open["session_id"].as_str().unwrap().to_string();

        r#loop::set_busy_for_tests(true);
        // Busy open must not swap Binding; keep tools ctx on `a`.
        let rejected = r#loop::open_ai_assistant_core(&b).unwrap();
        assert_eq!(rejected["busy"], true);
        assert!(rejected.get("bound_master_task_id").is_none());
        r#loop::set_busy_for_tests(false);

        let result = r#loop::agent_chat_turn_core(&sid, "改标题", Some(&a)).unwrap();
        assert_eq!(result.body["wrote"], true);
        assert_eq!(todo_task::get_by_id(&a)["title"], "落在旧绑定");
        assert_eq!(todo_task::get_by_id(&b)["title"], "新绑定");
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
fn set_binding_rejects_inapplicable_tools_or_prompt_as_set_invalid() {
    with_sandbox(|| {
        let empty_tools = r#loop::Binding {
            tools: json!([]),
            prompt: applicable_prompt(),
            callbacks: empty_callbacks_registry(),
        };
        let err = r#loop::set_binding(empty_tools).expect_err("empty tools must fail");
        assert_eq!(err.as_code(), "set_invalid");
        assert_eq!(r#loop::binding_state(), "unbound");

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
        let err = r#loop::set_binding(r#loop::Binding {
            tools: json!([]),
            prompt: applicable_prompt(),
            callbacks: empty_callbacks_registry(),
        })
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
        let q = query_binding_json();
        assert_eq!(q["state"], "unbound");
        let exec = execute_binding_json();
        assert_eq!(exec["ok"], false);
        assert_eq!(exec["code"], "rejected_unbound");

        // Present after Set still leaves bound and does not replace.
        let set = set_binding_json(serde_json::to_value(valid_binding()).unwrap());
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
