use rmcp::model::{
    CallToolRequest, CallToolRequestParams, ClientJsonRpcMessage, ClientRequest, GetMeta,
    NumberOrString, RequestMetaObject,
};
use serde_json::json;

use super::{build_trace_meta, read_trace_id, TRACE_META_KEY};
use crate::agent::diagnostics::TraceId;

#[test]
fn trace_id_round_trips_through_meta() {
    let trace_id = TraceId::new();

    let meta = build_trace_meta(&trace_id);

    assert_eq!(read_trace_id(&meta), Some(trace_id));
}

#[test]
fn missing_or_invalid_trace_id_reads_as_none() {
    assert_eq!(read_trace_id(&RequestMetaObject::new()), None);

    let mut invalid = RequestMetaObject::new();
    invalid.insert(TRACE_META_KEY.to_string(), json!("bad id\nwith newline"));
    assert_eq!(read_trace_id(&invalid), None);

    let mut wrong_type = RequestMetaObject::new();
    wrong_type.insert(TRACE_META_KEY.to_string(), json!(42));
    assert_eq!(read_trace_id(&wrong_type), None);
}

#[test]
fn trace_id_survives_the_json_rpc_wire_into_request_meta() {
    let trace_id = TraceId::new();
    let mut params = CallToolRequestParams::new("search");
    params.meta = Some(build_trace_meta(&trace_id));
    let request = ClientRequest::CallToolRequest(CallToolRequest::new(params));
    let message = ClientJsonRpcMessage::request(request, NumberOrString::Number(1));

    let wire = serde_json::to_value(&message).expect("serialize tools/call");
    assert_eq!(
        wire["params"]["_meta"][TRACE_META_KEY],
        json!(trace_id.as_str()),
        "client must put the trace id in params._meta"
    );

    // The server reads it from the request extensions (rmcp moves it into
    // `RequestContext::meta`), not from the typed params field.
    let decoded: ClientJsonRpcMessage = serde_json::from_value(wire).expect("deserialize");
    let (decoded_request, _) = decoded.into_request().expect("request message");
    assert_eq!(read_trace_id(decoded_request.get_meta()), Some(trace_id));
}
