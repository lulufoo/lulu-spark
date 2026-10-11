use rmcp::model::{
    CallToolRequest, CallToolRequestParams, ClientJsonRpcMessage, ClientRequest, GetMeta,
    NumberOrString, RequestMetaObject,
};
use serde_json::json;

use super::{build_trace_meta, read_trace_meta, TraceMeta, SESSION_META_KEY, TRACE_META_KEY};
use crate::agent::diagnostics::TraceId;

const SESSION: &str = "spark_chat_5141a723ad70f2cadd4e5885a5421b06";

#[test]
fn trace_and_session_ids_round_trip_through_meta() {
    let trace_id = TraceId::new();

    let meta = build_trace_meta(&trace_id, SESSION);

    assert_eq!(
        read_trace_meta(&meta),
        TraceMeta {
            trace_id: Some(trace_id),
            session_id: Some(SESSION.to_string()),
        }
    );
}

#[test]
fn empty_or_invalid_session_id_is_left_out_of_meta() {
    let trace_id = TraceId::new();

    for session_id in ["", "bad/session", "has space", "../escape"] {
        let meta = build_trace_meta(&trace_id, session_id);
        assert!(
            meta.get(SESSION_META_KEY).is_none(),
            "{session_id:?} must not be written"
        );
        assert_eq!(read_trace_meta(&meta).trace_id, Some(trace_id.clone()));
    }
}

#[test]
fn missing_or_invalid_ids_read_as_none() {
    assert_eq!(
        read_trace_meta(&RequestMetaObject::new()),
        TraceMeta::default()
    );

    let mut invalid = RequestMetaObject::new();
    invalid.insert(TRACE_META_KEY.to_string(), json!("bad id\nwith newline"));
    invalid.insert(SESSION_META_KEY.to_string(), json!("bad/session"));
    assert_eq!(read_trace_meta(&invalid), TraceMeta::default());

    let mut wrong_type = RequestMetaObject::new();
    wrong_type.insert(TRACE_META_KEY.to_string(), json!(42));
    wrong_type.insert(SESSION_META_KEY.to_string(), json!(7));
    assert_eq!(read_trace_meta(&wrong_type), TraceMeta::default());
}

#[test]
fn ids_survive_the_json_rpc_wire_into_request_meta() {
    let trace_id = TraceId::new();
    let mut params = CallToolRequestParams::new("search");
    params.meta = Some(build_trace_meta(&trace_id, SESSION));
    let request = ClientRequest::CallToolRequest(CallToolRequest::new(params));
    let message = ClientJsonRpcMessage::request(request, NumberOrString::Number(1));

    let wire = serde_json::to_value(&message).expect("serialize tools/call");
    assert_eq!(
        wire["params"]["_meta"][TRACE_META_KEY],
        json!(trace_id.as_str()),
        "client must put the trace id in params._meta"
    );
    assert_eq!(wire["params"]["_meta"][SESSION_META_KEY], json!(SESSION));

    // The server reads them from the request extensions (rmcp moves them into
    // `RequestContext::meta`), not from the typed params field.
    let decoded: ClientJsonRpcMessage = serde_json::from_value(wire).expect("deserialize");
    let (decoded_request, _) = decoded.into_request().expect("request message");
    let read = read_trace_meta(decoded_request.get_meta());
    assert_eq!(read.trace_id, Some(trace_id));
    assert_eq!(read.session_id.as_deref(), Some(SESSION));
}
