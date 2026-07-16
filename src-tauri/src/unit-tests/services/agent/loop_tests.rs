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
use crate::services::plan_task;
use crate::test_support::TestSandbox;

fn with_sandbox<F: FnOnce()>(f: F) {
    let _sandbox = TestSandbox::new();
    secrets::test_secrets_clear();
    r#loop::reset_runtime_for_tests();
    f();
}

fn create_bound_plan(title: &str) -> String {
    let created = plan_task::create_master_with_subs(title, Some(&["子项A"]));
    assert_eq!(created["_status"], 201);
    created["master_task_id"].as_str().unwrap().to_string()
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
        let messages = r#loop::build_llm_messages_from_turns(&turns);
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
        let out = r#loop::run_loop(&mut sess, "把主标题改成写后标题", &cfg_for(&mock));
        assert_outcome(&out, "none", true);
        let got = plan_task::get_by_id(&master);
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
        let out = r#loop::run_loop(&mut sess, "改不存在的子项并加一个", &cfg_for(&mock));
        assert_outcome(&out, "none", true);
        let tool_turns: Vec<_> = sess.turns.iter().filter(|t| t.role == "tool").collect();
        assert_eq!(tool_turns.len(), 2);
        let t1: Value = serde_json::from_str(tool_turns[0].content.as_deref().unwrap()).unwrap();
        let t2: Value = serde_json::from_str(tool_turns[1].content.as_deref().unwrap()).unwrap();
        assert_eq!(t1["ok"], false);
        assert_eq!(t2["ok"], true);
        let listed = plan_task::get_by_id(&master);
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
            out.reply_text.contains("无计划") || out.reply_text.contains("没有"),
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
        let before = plan_task::get_by_id(&master)["title"].clone();
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
        let out = r#loop::run_loop(&mut sess, "删掉计划", &cfg_for(&mock));
        assert_outcome(&out, "error", false);
        assert_eq!(plan_task::get_by_id(&master)["title"], before);
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
        let listed = plan_task::get_by_id(&master);
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

        let out_add = r#loop::run_loop(&mut sess, "加一个子计划叫新观察子项", &cfg_for(&mock));
        assert_outcome(&out_add, "none", true);
        let titles_after_add: Vec<_> = plan_task::get_by_id(&master)["sub_tasks"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| s["title"].as_str().unwrap().to_string())
            .collect();
        assert!(titles_after_add.iter().any(|t| t == "新观察子项"));

        let out_upd = r#loop::run_loop(&mut sess, "把原子项标题改成改后子标题", &cfg_for(&mock));
        assert_outcome(&out_upd, "none", true);
        let updated = plan_task::get_by_id(&master);
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
            no_plan.reply_text.contains("无计划") || no_plan.reply_text.contains("没有"),
            "{}",
            no_plan.reply_text
        );

        let master = create_bound_plan("分型");
        let mock_unsup = spawn_scripted_llm(vec![assistant_text(
            "目前不支持删除计划，我只能查看、加子计划或改标题。",
        )]);
        let mut sess = session::create_session(Some(&master), Some("分型")).unwrap();
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
        let before_subs = plan_task::get_by_id(&master)["sub_tasks"]
            .as_array()
            .unwrap()
            .len();
        let mock = spawn_scripted_llm(vec![(200, json!({}))]);
        let mut sess = session::create_session(Some(&master), Some("畸形")).unwrap();
        let out = r#loop::run_loop(&mut sess, "加子项", &cfg_for(&mock));
        assert_outcome(&out, "error", false);
        assert_eq!(
            plan_task::get_by_id(&master)["sub_tasks"]
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
        assert_eq!(v["bound_master_task_id"], master);
        assert_eq!(v["bound_title"], "打开助手");
        assert_eq!(v["window_label"], "ai-assistant");
        assert_ne!(v["busy"], true);
    });
}

#[test]
fn open_ai_assistant_busy_rejects_rebind() {
    with_sandbox(|| {
        let a = create_bound_plan("计划A");
        let b = create_bound_plan("计划B");
        let first = r#loop::open_ai_assistant_core(&a).unwrap();
        let sid = first["session_id"].as_str().unwrap().to_string();
        r#loop::set_busy_for_tests(true);
        let second = r#loop::open_ai_assistant_core(&b).unwrap();
        assert_eq!(second["busy"], true);
        assert_eq!(second["session_id"], sid);
        assert_eq!(second["bound_master_task_id"], a);
        assert_eq!(second["bound_title"], "计划A");
    });
}

#[test]
fn agent_chat_turn_happy_path_emits_turn_completed() {
    with_sandbox(|| {
        let master = create_bound_plan("对话");
        let mock = spawn_scripted_llm(vec![assistant_text("收到")]);
        install_llm_cfg(&mock);
        let open = r#loop::open_ai_assistant_core(&master).unwrap();
        let sid = open["session_id"].as_str().unwrap();
        let result = r#loop::agent_chat_turn_core(sid, "你好", Some(&master)).unwrap();
        assert_eq!(result.body["terminal"], "none");
        assert_eq!(result.body["wrote"], false);
        assert_eq!(result.body["busy"], false);
        assert_eq!(result.body["bound_master_task_id"], master);
        assert!(result.body["reply_text"].as_str().unwrap().contains("收到"));
        let ev = result.emit_turn_completed.expect("must emit");
        assert_eq!(ev["event"], EVENT_TURN_COMPLETED);
        assert_eq!(ev["payload"]["session_id"], sid);
        assert_eq!(ev["payload"]["wrote"], false);
        assert_eq!(ev["payload"]["terminal"], "none");
        assert_eq!(ev["payload"]["bound_master_task_id"], master);
    });
}

#[test]
fn agent_chat_turn_busy_rejects_without_emit_or_user_turn() {
    with_sandbox(|| {
        let master = create_bound_plan("忙");
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
fn agent_chat_turn_master_mismatch_is_business_no_llm() {
    with_sandbox(|| {
        let a = create_bound_plan("绑定A");
        let b = create_bound_plan("其它B");
        let mock = spawn_scripted_llm(vec![assistant_text("nope")]);
        install_llm_cfg(&mock);
        let open = r#loop::open_ai_assistant_core(&a).unwrap();
        let sid = open["session_id"].as_str().unwrap();
        let result = r#loop::agent_chat_turn_core(sid, "你好", Some(&b)).unwrap();
        assert_eq!(result.body["terminal"], "business");
        assert_eq!(result.body["wrote"], false);
        assert!(result.emit_turn_completed.is_some());
        assert_eq!(mock.hits.lock().unwrap().len(), 0);
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
        let open = r#loop::open_ai_assistant_core(&a).unwrap();
        let sid = open["session_id"].as_str().unwrap().to_string();

        r#loop::set_busy_for_tests(true);
        let rejected = r#loop::open_ai_assistant_core(&b).unwrap();
        assert_eq!(rejected["busy"], true);
        assert_eq!(rejected["bound_master_task_id"], a);
        r#loop::set_busy_for_tests(false);

        let result = r#loop::agent_chat_turn_core(&sid, "改标题", Some(&a)).unwrap();
        assert_eq!(result.body["wrote"], true);
        assert_eq!(plan_task::get_by_id(&a)["title"], "落在旧绑定");
        assert_eq!(plan_task::get_by_id(&b)["title"], "新绑定");
    });
}

#[test]
fn close_window_does_not_abort_emit_still_available() {
    with_sandbox(|| {
        let master = create_bound_plan("关窗");
        let mock = spawn_scripted_llm(vec![assistant_text("跑完了")]);
        install_llm_cfg(&mock);
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
