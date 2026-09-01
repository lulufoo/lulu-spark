//! `run_loop` tool-round, terminal, and Host empty-tools tests.

use super::support::*;


#[test]
fn run_loop_final_reply_none_terminal_and_wrote_false() {
    with_sandbox(|| {
        let mock = spawn_scripted_llm(vec![assistant_text("你好，我是计划助手")]);
        let master = create_bound_plan("主计划");
        let mut sess = session::create_session().unwrap();
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
fn run_loop_uses_mcp_tools_and_feeds_tool_result_back_to_model() {
    let sandbox = TestSandbox::new();
    secrets::test_secrets_clear();
    r#loop::reset_runtime_for_tests();
    mcp_registry::clear_for_tests();
    mcp_registry::seed_defaults();

    let todo_root = sandbox.workbench_root().join("todo_tasks");
    fs::create_dir_all(&todo_root).expect("create todo root");
    fs::write(todo_root.join(".migration_gate_passed"), b"ok\n").expect("plant migration gate");

    let mcp_port = ephemeral_port();
    let mcp = start_embedded_mcp_runtime(McpRuntimeConfig {
        bind_addr: format!("127.0.0.1:{mcp_port}").parse().expect("MCP address"),
    })
    .expect("start isolated MCP");
    mcp_registry::register(
        "workbench",
        McpServerConfig {
            capability_description: "workbench test capability".into(),
            http_transport: HttpMcpTransport {
                name: "workbench-test".into(),
                url: format!("http://127.0.0.1:{mcp_port}/mcp/workbench"),
                headers: bearer_headers_for_slot("workbench"),
            },
        },
    )
    .expect("register workbench MCP");

    let mock = spawn_scripted_llm(vec![
        assistant_tools(
            json!([{
                "id": "selection_1",
                "type": "function",
                "function": {
                    "name": "list_todo_tasks",
                    "arguments": "{}"
                }
            }]),
            None,
        ),
        assistant_text("已读取当前待办列表。"),
    ]);
    r#loop::try_set_binding_json(&json!({ "key": "workbench" })).expect("Set workbench binding");
    let mut session = session::create_session().expect("session");
    let outcome = r#loop::run_loop(&mut session, "读取当前待办", &cfg_for(&mock));

    assert_outcome(&outcome, "none", false);
    assert_eq!(
        session
            .turns
            .iter()
            .filter(|turn| turn.role == "tool")
            .count(),
        1,
        "MCP tool result must be persisted as a role=tool turn"
    );
    let hits = mock.hits.lock().unwrap();
    assert_eq!(hits.len(), 2, "tool result must trigger an LLM follow-up");
    assert!(
        hits[0]["tools"]
            .as_array()
            .is_some_and(|tools| tools.iter().any(|tool| {
                tool.pointer("/function/name") == Some(&json!("list_todo_tasks"))
            })),
        "active workbench MCP tools must be sent to the model"
    );
    assert!(
        hits[1]["messages"]
            .as_array()
            .is_some_and(|messages| messages.iter().any(|message| {
                message["role"] == "tool"
                    && message["tool_call_id"] == "selection_1"
                    && message["name"] == "list_todo_tasks"
            })),
        "the model follow-up must receive the MCP result"
    );

    drop(hits);
    stop_embedded_mcp_runtime(mcp).expect("stop MCP");
}

#[test]
fn run_loop_marks_successful_todo_mcp_mutation_as_wrote() {
    let sandbox = TestSandbox::new();
    secrets::test_secrets_clear();
    r#loop::reset_runtime_for_tests();
    mcp_registry::clear_for_tests();
    mcp_registry::seed_defaults();

    let todo_root = sandbox.workbench_root().join("todo_tasks");
    fs::create_dir_all(&todo_root).expect("create todo root");
    fs::write(todo_root.join(".migration_gate_passed"), b"ok\n").expect("plant migration gate");
    let mcp_port = ephemeral_port();
    let mcp = start_embedded_mcp_runtime(McpRuntimeConfig {
        bind_addr: format!("127.0.0.1:{mcp_port}").parse().expect("MCP address"),
    })
    .expect("start isolated MCP");
    mcp_registry::register(
        "workbench",
        McpServerConfig {
            capability_description: "todo test capability".into(),
            http_transport: HttpMcpTransport {
                name: "workbench-test".into(),
                url: format!("http://127.0.0.1:{mcp_port}/mcp/workbench"),
                headers: bearer_headers_for_slot("workbench"),
            },
        },
    )
    .expect("register workbench MCP");

    let mock = spawn_scripted_llm(vec![
        assistant_tools(
            json!([{
                "id": "create_1",
                "type": "function",
                "function": {
                    "name": "create_todo_task",
                    "arguments": "{\"title\":\"MCP 创建任务\",\"todo_md\":\"MCP body\"}"
                }
            }]),
            None,
        ),
        assistant_text("已创建任务。"),
    ]);
    r#loop::try_set_binding_json(&json!({ "key": "workbench" })).expect("Set workbench binding");
    let mut session = session::create_session().expect("session");
    let outcome = r#loop::run_loop(&mut session, "创建一个任务", &cfg_for(&mock));

    assert_outcome(&outcome, "none", true);
    assert!(
        todo_task::list_all().expect("todo").as_array().is_some_and(|tasks| {
            tasks
                .iter()
                .any(|task| task["title"] == "MCP 创建任务")
        }),
        "a successful MCP tool mutation must reach Services"
    );
    let hits = mock.hits.lock().unwrap();
    assert!(
        hits[0]["tools"]
            .as_array()
            .is_some_and(|tools| tools.iter().any(|tool| {
                tool.pointer("/function/name") == Some(&json!("create_todo_task"))
            })),
        "Todo tool definition must reach the model"
    );

    drop(hits);
    stop_embedded_mcp_runtime(mcp).expect("stop MCP");
}

#[test]
fn run_loop_returns_argument_and_allowlist_failures_to_the_model_as_tool_turns() {
    let sandbox = TestSandbox::new();
    secrets::test_secrets_clear();
    r#loop::reset_runtime_for_tests();
    mcp_registry::clear_for_tests();
    mcp_registry::seed_defaults();
    let (mcp, mcp_port) = start_isolated_mcp(&sandbox);
    register_test_mcp("workbench", mcp_port);

    let mock = spawn_scripted_llm(vec![
        assistant_tools(
            json!([
                {
                    "id": "invalid_args",
                    "type": "function",
                    "function": {
                        "name": "get_note_digest_by_id",
                        "arguments": "{}"
                    }
                },
                {
                    "id": "unknown_tool",
                    "type": "function",
                    "function": {
                        "name": "not_exposed_by_notes",
                        "arguments": "{}"
                    }
                }
            ]),
            None,
        ),
        assistant_text("工具参数或权限不正确，未执行读取。"),
    ]);
    r#loop::try_set_binding_json(&json!({ "key": "workbench" })).expect("Set workbench binding");
    let mut session = session::create_session().expect("session");
    let outcome = r#loop::run_loop(&mut session, "读取选择", &cfg_for(&mock));

    assert_outcome(&outcome, "none", false);
    let tool_turns: Vec<_> = session
        .turns
        .iter()
        .filter(|turn| turn.role == "tool")
        .collect();
    assert_eq!(tool_turns.len(), 2);
    assert!(
        tool_turns[0]
            .content
            .as_deref()
            .is_some_and(|content| content.contains("missing required property 'id'")),
        "arguments that violate the published schema must be rejected locally"
    );
    assert!(
        tool_turns[1]
            .content
            .as_deref()
            .is_some_and(|content| content.contains("not available in the active scene")),
        "undiscovered tools must never be sent through MCP"
    );
    assert!(
        mock.hits.lock().unwrap()[1]["messages"]
            .as_array()
            .is_some_and(|messages| messages.iter().filter(|message| message["role"] == "tool").count() == 2),
        "both failures must be recoverable by the model on the next round"
    );

    stop_embedded_mcp_runtime(mcp).expect("stop MCP");
}

#[test]
fn run_loop_stops_after_bounded_mcp_tool_rounds() {
    let sandbox = TestSandbox::new();
    secrets::test_secrets_clear();
    r#loop::reset_runtime_for_tests();
    mcp_registry::clear_for_tests();
    mcp_registry::seed_defaults();
    let (mcp, mcp_port) = start_isolated_mcp(&sandbox);
    register_test_mcp("workbench", mcp_port);

    let responses = (0..=r#loop::MAX_MCP_TOOL_ROUNDS)
        .map(|round| {
            assistant_tools(
                json!([{
                    "id": format!("selection_{round}"),
                    "type": "function",
                    "function": {
                        "name": "list_todo_tasks",
                        "arguments": "{}"
                    }
                }]),
                None,
            )
        })
        .collect();
    let mock = spawn_scripted_llm(responses);
    r#loop::try_set_binding_json(&json!({ "key": "workbench" })).expect("Set workbench binding");
    let mut session = session::create_session().expect("session");
    let outcome = r#loop::run_loop(&mut session, "反复读取待办", &cfg_for(&mock));

    assert_outcome(&outcome, "error", false);
    assert!(outcome.reply_text.contains("调用次数已达上限"));
    assert_eq!(
        session
            .turns
            .iter()
            .filter(|turn| turn.role == "tool")
            .count(),
        r#loop::MAX_MCP_TOOL_ROUNDS,
        "the cap must prevent the next requested tool call"
    );
    assert_eq!(
        mock.hits.lock().unwrap().len(),
        r#loop::MAX_MCP_TOOL_ROUNDS + 1,
        "the cap is checked after the model requests its next tool round"
    );

    stop_embedded_mcp_runtime(mcp).expect("stop MCP");
}

#[test]
fn run_loop_offers_host_file_tools_and_keeps_scratch_writes_inside_cache() {
    let sandbox = TestSandbox::new();
    secrets::test_secrets_clear();
    r#loop::reset_runtime_for_tests();
    mcp_registry::clear_for_tests();
    mcp_registry::seed_defaults();
    let readable = sandbox.workbench_root().join("readable.md");
    fs::write(&readable, "needle-line\n").expect("plant readable file");
    let forbidden = sandbox.workbench_root().join("todo.md");
    let (mcp, mcp_port) = start_isolated_mcp(&sandbox);
    register_test_mcp("workbench", mcp_port);

    r#loop::try_set_binding_json(&json!({ "key": "workbench" })).expect("Set workbench binding");
    let mut session = session::create_session().expect("session");
    let scratch = sandbox
        .cache_dir()
        .join("agent-scratch")
        .join(&session.session_id)
        .join("pad.md");
    let mock = spawn_scripted_llm(vec![
        assistant_tools(
            json!([{
                "id": "host_read",
                "type": "function",
                "function": {
                    "name": "read",
                    "arguments": format!("{{\"path\":{}}}", json!(readable.to_string_lossy()))
                }
            }]),
            None,
        ),
        assistant_tools(
            json!([{
                "id": "host_denied",
                "type": "function",
                "function": {
                    "name": "write",
                    "arguments": format!(
                        "{{\"path\":{},\"content\":\"nope\"}}",
                        json!(forbidden.to_string_lossy())
                    )
                }
            }]),
            None,
        ),
        assistant_tools(
            json!([{
                "id": "host_write",
                "type": "function",
                "function": {
                    "name": "write",
                    "arguments": format!(
                        "{{\"path\":{},\"content\":\"scratch ok\"}}",
                        json!(scratch.to_string_lossy())
                    )
                }
            }]),
            None,
        ),
        assistant_text("已读文件并写入草稿。"),
    ]);
    let outcome = r#loop::run_loop(&mut session, "读文件并写草稿", &cfg_for(&mock));

    assert_outcome(&outcome, "none", true);
    assert!(!forbidden.exists(), "Host write must not touch $WB");
    assert_eq!(
        fs::read_to_string(&scratch).expect("scratch"),
        "scratch ok"
    );
    let hits = mock.hits.lock().unwrap();
    assert!(
        hits[0]["tools"].as_array().is_some_and(|tools| {
            ["grep", "read", "write", "edit"].iter().all(|name| {
                tools
                    .iter()
                    .any(|tool| tool.pointer("/function/name") == Some(&json!(name)))
            }) && tools.iter().any(|tool| {
                tool.pointer("/function/name") == Some(&json!("list_todo_tasks"))
            })
        }),
        "model tools must include Host file tools and MCP tools"
    );
    let tool_turns: Vec<_> = session
        .turns
        .iter()
        .filter(|turn| turn.role == "tool")
        .collect();
    assert_eq!(tool_turns.len(), 3);
    assert!(
        tool_turns[0]
            .content
            .as_deref()
            .is_some_and(|content| content.contains("needle-line")),
        "Host read must return the $WB file"
    );
    assert!(
        tool_turns[1]
            .content
            .as_deref()
            .is_some_and(|content| content.contains("outside the write fence")),
        "Host write outside scratch must be denied"
    );
    let log = fs::read_to_string(
        crate::agent::diagnostics::diagnostic_log_path().expect("diag path"),
    )
    .unwrap_or_default();
    assert!(
        log.contains("\"tool_name\":\"read\"")
            && log.contains("\"source\":\"host\"")
            && log.contains(&format!("\"session_id\":\"{}\"", session.session_id)),
        "Host file tools must reuse assistant-diagnostic.jsonl with session_id"
    );

    drop(hits);
    stop_embedded_mcp_runtime(mcp).expect("stop MCP");
}

#[test]
fn run_loop_does_not_call_mcp_after_reset_invalidates_its_generation() {
    let sandbox = TestSandbox::new();
    secrets::test_secrets_clear();
    r#loop::reset_runtime_for_tests();
    mcp_registry::clear_for_tests();
    mcp_registry::seed_defaults();
    let (mcp, mcp_port) = start_isolated_mcp(&sandbox);
    register_test_mcp("workbench", mcp_port);

    let mock = spawn_llm_with_mid_then_response(
        || r#loop::reset_binding().expect("Reset while LLM response is in flight"),
        assistant_tools(
            json!([{
                "id": "selection_after_reset",
                "type": "function",
                "function": {
                    "name": "list_todo_tasks",
                    "arguments": "{}"
                }
            }]),
            None,
        ),
    );
    r#loop::try_set_binding_json(&json!({ "key": "workbench" })).expect("Set workbench binding");
    let mut session = session::create_session().expect("session");
    let outcome = r#loop::run_loop(&mut session, "读取待办", &cfg_for(&mock));

    assert_outcome(&outcome, "error", false);
    assert!(outcome.reply_text.contains("cancelled"));
    assert!(
        session.turns.is_empty(),
        "cut cancellation must truncate the old generation's user and tool turns"
    );
    assert_eq!(
        mock.hits.lock().unwrap().len(),
        1,
        "the LLM may return once, but MCP tools must not be called after Reset"
    );

    stop_embedded_mcp_runtime(mcp).expect("stop MCP");
}

#[test]
fn run_loop_host_empty_tools_rejects_tool_calls_without_dispatch() {
    // Narrowed from process-local tool write path (P3 / T3): Host business chat
    // sends tools=[] and must not tools::dispatch even if the model returns tool_calls.
    with_sandbox(|| {
        let master = create_bound_plan("写前标题");
        let title_before = todo_task::get_by_id(&master).expect("todo")["title"].clone();
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
        let mut sess = session::create_session().unwrap();
        arm_plan_binding(&master);
        let out = r#loop::run_loop(&mut sess, "把主标题改成写后标题", &cfg_for(&mock));
        assert_outcome(&out, "error", false);
        assert_eq!(todo_task::get_by_id(&master).expect("todo")["title"], title_before);
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
        let mut sess = session::create_session().unwrap();
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
        let before_subs = todo_task::get_by_id(&master).expect("todo")["sub_tasks"]
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
        let mut sess = session::create_session().unwrap();
        arm_plan_binding(&master);
        let out = r#loop::run_loop(&mut sess, "改不存在的子项并加一个", &cfg_for(&mock));
        assert_outcome(&out, "error", false);
        assert_eq!(
            todo_task::get_by_id(&master).expect("todo")["sub_tasks"]
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
        let before_subs = todo_task::get_by_id(&master).expect("todo")["sub_tasks"]
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
        let mut sess = session::create_session().unwrap();
        arm_plan_binding(&master);
        let out = r#loop::run_loop(&mut sess, "加子项", &cfg_for(&mock));
        assert_outcome(&out, "error", false);
        assert_ne!(out.reply_text, "已新增同条子项");
        assert_eq!(
            todo_task::get_by_id(&master).expect("todo")["sub_tasks"]
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
        let mut sess = session::create_session().unwrap();
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
        let before = todo_task::get_by_id(&master).expect("todo")["title"].clone();
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
        let mut sess = session::create_session().unwrap();
        arm_plan_binding(&master);
        let out = r#loop::run_loop(&mut sess, "删掉计划", &cfg_for(&mock));
        assert_outcome(&out, "error", false);
        assert_eq!(todo_task::get_by_id(&master).expect("todo")["title"], before);
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
        let mut sess = session::create_session().unwrap();
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
            let mut sess = session::create_session().unwrap();
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
        let before = todo_task::get_by_id(&master).expect("todo");
        let before_sub_title = before["sub_tasks"][0]["title"].clone();
        let before_len = before["sub_tasks"].as_array().unwrap().len();
        let mock = spawn_scripted_llm(vec![
            assistant_text("已记录新增子计划的请求（Host 本阶段不经 tool_calls 写入）"),
            assistant_text("已记录改子标题的请求（Host 本阶段不经 tool_calls 写入）"),
        ]);
        let mut sess = session::create_session().unwrap();
        arm_plan_binding(&master);

        let out_add = r#loop::run_loop(&mut sess, "加一个子计划叫新观察子项", &cfg_for(&mock));
        assert_outcome(&out_add, "none", false);
        assert_eq!(
            todo_task::get_by_id(&master).expect("todo")["sub_tasks"]
                .as_array()
                .unwrap()
                .len(),
            before_len
        );

        let out_upd = r#loop::run_loop(&mut sess, "把原子项标题改成改后子标题", &cfg_for(&mock));
        assert_outcome(&out_upd, "none", false);
        assert_eq!(
            todo_task::get_by_id(&master).expect("todo")["sub_tasks"][0]["title"],
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
        let mut unbound = session::create_session().unwrap();
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
        let mut sess = session::create_session().unwrap();
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
        let mut sess2 = session::create_session().unwrap();
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
        let before_subs = todo_task::get_by_id(&master).expect("todo")["sub_tasks"]
            .as_array()
            .unwrap()
            .len();
        let mock = spawn_scripted_llm(vec![(200, json!({}))]);
        let mut sess = session::create_session().unwrap();
        arm_plan_binding(&master);
        let out = r#loop::run_loop(&mut sess, "加子项", &cfg_for(&mock));
        assert_outcome(&out, "error", false);
        assert_eq!(
            todo_task::get_by_id(&master).expect("todo")["sub_tasks"]
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
        let mut sess = session::create_session().unwrap();
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
        let mut sess = session::create_session().unwrap();
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
        let mut sess = session::create_session().unwrap();
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
fn t1_run_loop_aborts_tool_dispatch_when_generation_invalidated() {
    with_sandbox(|| {
        let master = create_bound_plan("t1-gen-tools");
        let title_before = todo_task::get_by_id(&master).expect("todo")["title"]
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
        let open = r#loop::ensure_chat_session_core().unwrap();
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
            todo_task::get_by_id(&master).expect("todo")["title"],
            title_before,
            "old Tools must not write after generation invalidation"
        );
    });
}

#[test]
fn t3_mid_chat_reset_cancels_without_appending_business_turns() {
    with_sandbox(|| {
        let master = create_bound_plan("t3-chat-reset");
        let title_before = todo_task::get_by_id(&master).expect("todo")["title"]
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
        let open = r#loop::ensure_chat_session_core().unwrap();
        let sid = open["session_id"].as_str().unwrap().to_string();
        let turns_before = session_turn_contents(&sid);

        let result = r#loop::agent_chat_turn_core(&sid, "改标题", Some(&master)).unwrap();
        assert_eq!(result.body["wrote"], false);
        assert_eq!(
            result.body["terminal"], "error",
            "cancelled in-flight chat must stop as error, not fly the round to success"
        );
        assert_eq!(
            todo_task::get_by_id(&master).expect("todo")["title"],
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
        let title_before = todo_task::get_by_id(&master_a).expect("todo")["title"]
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
        let open = r#loop::ensure_chat_session_core().unwrap();
        let sid = open["session_id"].as_str().unwrap().to_string();
        let turns_before = session_turn_contents(&sid);

        let result = r#loop::agent_chat_turn_core(&sid, "改标题", Some(&master_a)).unwrap();
        assert_eq!(result.body["wrote"], false);
        assert_eq!(result.body["terminal"], "error");
        assert_eq!(todo_task::get_by_id(&master_a).expect("todo")["title"], title_before);
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
        let title_before = todo_task::get_by_id(&master).expect("todo")["title"]
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
        let open = r#loop::ensure_chat_session_core().unwrap();
        let sid = open["session_id"].as_str().unwrap().to_string();
        let turns_before = session_turn_contents(&sid);

        let result = r#loop::agent_chat_turn_core(&sid, "改标题", Some(&master)).unwrap();
        assert!(
            r#loop::is_binding_generation_current(gen),
            "this case keeps generation current; stop must be from cancel flag"
        );
        assert_eq!(result.body["wrote"], false);
        assert_eq!(result.body["terminal"], "error");
        assert_eq!(todo_task::get_by_id(&master).expect("todo")["title"], title_before);
        assert_eq!(
            session_turn_contents(&sid),
            turns_before,
            "cancel-flag reject must not append business turns"
        );
    });
}

// --- T3 / P3: Host Loop empty tools ---

#[test]
fn t3_host_business_chat_sends_empty_tools_to_llm() {
    with_sandbox(|| {
        let master = create_bound_plan("t3-empty-tools");
        let mock = spawn_scripted_llm(vec![assistant_text("门面会话可用")]);
        install_llm_cfg(&mock);
        arm_plan_binding(&master);
        let open = r#loop::ensure_chat_session_core().unwrap();
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
fn t3_host_path_modules_do_not_import_removed_agent_stack() {
    // Module boundary: Host loop/llm must not import the removed agent stack.
    let loop_src = LOOP_SRC;
    let llm_src = include_str!("../../../agent/llm.rs");
    for (label, src) in [("loop.rs", loop_src), ("llm.rs", llm_src)] {
        let lower = src.to_ascii_lowercase();
        assert!(
            !lower.contains("process_manager"),
            "{label} must not import/link process_manager"
        );
        assert!(
            !lower.contains("cursor_sdk"),
            "{label} must not import/link removed SDK code"
        );
        assert!(
            !src.contains("cursor_agent") && !src.contains("CursorAgent"),
            "{label} must not reference removed agent symbols"
        );
    }
}

#[test]
fn t3_host_reports_unavailable_loaded_mcp_without_dispatch() {
    with_sandbox(|| {
        use crate::mcp_host::registry::SEEDED_BUSINESS_KEY;
        let mock = spawn_scripted_llm(vec![assistant_text("读只读面后文本回复")]);
        install_llm_cfg(&mock);
        r#loop::try_set_binding_json(&key_only_payload(SEEDED_BUSINESS_KEY)).expect("Set");
        let before = r#loop::session_capability_mcp_config().expect("loaded");

        // A key-only binding must fail before the LLM when its configured MCP
        // endpoint is unavailable, without mutating the loaded capability.
        let open = r#loop::ensure_chat_session_core().unwrap();
        let sid = open["session_id"].as_str().unwrap();
        let result = r#loop::agent_chat_turn_core(sid, "ping", None).unwrap();
        assert_eq!(result.body["terminal"], "error");
        assert_eq!(result.body["wrote"], false);
        assert!(
            result.body["reply_text"]
                .as_str()
                .is_some_and(|text| text.contains("MCP")),
            "unavailable MCP must be reported to the user"
        );

        let after = r#loop::session_capability_mcp_config().expect("still loaded");
        assert_eq!(after, before, "chat must not rewrite session capability MCP config");
        assert!(mock.hits.lock().unwrap().is_empty(), "LLM must not run without MCP tools");
    });
}

#[test]
fn t3_host_unexpected_tool_calls_never_call_process_dispatch() {
    with_sandbox(|| {
        let master = create_bound_plan("t3-no-dispatch");
        let title_before = todo_task::get_by_id(&master).expect("todo")["title"].clone();
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
        let open = r#loop::ensure_chat_session_core().unwrap();
        let sid = open["session_id"].as_str().unwrap();
        let result = r#loop::agent_chat_turn_core(sid, "改标题", Some(&master)).unwrap();
        assert_eq!(result.body["wrote"], false);
        assert_eq!(result.body["terminal"], "error");
        assert_eq!(todo_task::get_by_id(&master).expect("todo")["title"], title_before);
        let sess = session::load_session(sid).unwrap();
        assert!(sess.turns.iter().all(|t| t.role != "tool"));
        assert_host_llm_tools_empty(&mock.hits.lock().unwrap()[0]);
    });
}

#[test]
fn t3_host_empty_tools_facade_usable_confirmed() {
    assert!(
        r#loop::HOST_EMPTY_TOOLS_FACADE_USABLE,
        "Host empty tools must remain usable for facade sessions"
    );
    with_sandbox(|| {
        let master = create_bound_plan("t3-a3");
        let mock = spawn_scripted_llm(vec![assistant_text("A3: empty tools facade ok")]);
        install_llm_cfg(&mock);
        arm_plan_binding(&master);
        let open = r#loop::ensure_chat_session_core().unwrap();
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
    let src = LOOP_SRC;
    // Legacy in-process business dispatch is gone; turn routes via tools::invoke.
    assert!(
        !src.contains("tools::dispatch("),
        "loop.rs must not call legacy tools::dispatch"
    );
    assert!(
        src.contains("tools::discover_and_merge") && src.contains("tools::invoke"),
        "loop.rs must prepare and dispatch tools via tools::"
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
        let before = todo_task::get_by_id(&master).expect("todo")["title"].clone();
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
        let mut sess = session::create_session().unwrap();
        arm_plan_binding(&master);
        let out = r#loop::run_loop(&mut sess, "改标题", &cfg_for(&mock));
        assert_eq!(out.wrote, false);
        assert_eq!(todo_task::get_by_id(&master).expect("todo")["title"], before);
        assert!(
            !sess.turns.iter().any(|t| t.role == "tool"),
            "no tool-role turns from removed dispatch path"
        );
        assert_eq!(out.terminal, Terminal::Error);
        assert_host_llm_tools_empty(&mock.hits.lock().unwrap()[0]);
    });
}

#[test]
fn t2_empty_tools_binding_text_only_round_succeeds() {
    with_sandbox(|| {
        let master = create_bound_plan("空tools纯文本");
        let mock = spawn_scripted_llm(vec![assistant_text("纯文本回复，无工具")]);
        let mut sess = session::create_session().unwrap();
        r#loop::set_binding(empty_tools_binding()).expect("empty tools Set");
        let out = r#loop::run_loop(&mut sess, "你好", &cfg_for(&mock));
        assert_outcome(&out, "none", false);
        assert!(out.reply_text.contains("纯文本"));
        assert_host_llm_tools_empty(&mock.hits.lock().unwrap()[0]);
    });
}

#[test]
fn t2_key_only_set_loads_mcp_and_rejects_unreachable_endpoint() {
    with_sandbox(|| {
        use crate::mcp_host::registry::SEEDED_BUSINESS_KEY;
        let master = create_bound_plan("key-only空tools");
        let mock = spawn_scripted_llm(vec![assistant_text("key-only工具回复")]);
        let mut sess = session::create_session().unwrap();
        r#loop::try_set_binding_json(&key_only_payload(SEEDED_BUSINESS_KEY))
            .expect("key-only Set");
        assert!(r#loop::loaded_mcp_server().is_some());
        assert!(r#loop::session_capability_mcp_config().is_some());
        let before = todo_task::get_by_id(&master).expect("todo")["title"].clone();
        let out = r#loop::run_loop(&mut sess, "你好", &cfg_for(&mock));
        assert_outcome(&out, "error", false);
        assert_eq!(out.wrote, false);
        assert!(
            !sess.turns.iter().any(|t| t.role == "tool"),
            "key-only business path must not dispatch in-process tools"
        );
        assert_eq!(todo_task::get_by_id(&master).expect("todo")["title"], before);
        assert!(
            mock.hits.lock().unwrap().is_empty(),
            "the LLM must not receive an empty-tools fallback for a key-only MCP binding"
        );
    });
}

#[test]
fn run_loop_emits_requesting_progress() {
    with_sandbox(|| {
        let master = create_bound_plan("进度");
        let mock = spawn_scripted_llm(vec![assistant_text("收到")]);
        let mut sess = session::create_session().unwrap();
        r#loop::set_binding(empty_tools_binding()).expect("empty tools Set");
        let collected = Arc::new(Mutex::new(Vec::new()));
        let slot = collected.clone();
        let sink: crate::agent::progress::ProgressSink =
            Arc::new(move |desc| slot.lock().unwrap().push(desc.desc));
        let out = r#loop::run_loop_with_progress(
            &mut sess,
            "你好",
            &cfg_for(&mock),
            &crate::agent::diagnostics::TraceId::new(),
            Some(&sink),
        );
        assert_outcome(&out, "none", false);
        assert!(
            collected
                .lock()
                .unwrap()
                .iter()
                .any(|desc| desc == "Requesting…"),
            "loop must emit Requesting…"
        );
    });
}
