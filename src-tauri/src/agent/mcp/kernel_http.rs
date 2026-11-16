//! The MCP transport's HTTP backend (LW-814): a reqwest 0.13 client from the
//! network kernel, snapshotted when the transport is created (LW-809 D6).
//! Request behavior — auth, protocol headers, session handling, SSE size
//! limits — is rmcp's own reqwest implementation; this wrapper only supplies
//! the client, so a live session keeps its snapshot and a reconnect or new
//! session picks up the current plan. Switching never resets the connection.

use std::collections::HashMap;
use std::sync::Arc;

use axum::http::{HeaderName, HeaderValue};
use futures::stream::BoxStream;
use rmcp::model::ClientJsonRpcMessage;
use rmcp::transport::streamable_http_client::{
    SseError, StreamableHttpClient, StreamableHttpPostResponse, StreamableHttpError,
};
use sse_stream::Sse;

use super::McpError;
use crate::net::{self, ClientSpec, RedirectSpec};

/// The transport spec mirrors the SDK default client: redirects disabled so
/// caller-supplied headers cannot leak to a redirect target, pooling off,
/// no timeouts (an SSE stream lives as long as the server keeps it).
fn transport_spec() -> ClientSpec {
    ClientSpec::default()
        .with_redirect(RedirectSpec::None)
        .with_pool_max_idle_per_host(0)
}

/// One client snapshot from the kernel, taken at transport-creation time.
pub(super) fn transport_client() -> Result<reqwest013::Client, McpError> {
    net::kernel()
        .async_external(&transport_spec())
        .map_err(|e| McpError::Connect(e.to_string()))
}

/// A rmcp HTTP backend over one kernel client snapshot. Every call forwards
/// to rmcp's implementation for `reqwest::Client`, including the
/// `max_sse_event_size` variants the transport drives.
#[derive(Clone)]
pub(super) struct KernelHttpClient {
    client: reqwest013::Client,
}

impl KernelHttpClient {
    pub(super) fn new(client: reqwest013::Client) -> Self {
        Self { client }
    }
}

impl StreamableHttpClient for KernelHttpClient {
    type Error = reqwest013::Error;

    async fn post_message(
        &self,
        uri: Arc<str>,
        message: ClientJsonRpcMessage,
        session_id: Option<Arc<str>>,
        auth_header: Option<String>,
        custom_headers: HashMap<HeaderName, HeaderValue>,
    ) -> Result<StreamableHttpPostResponse, StreamableHttpError<Self::Error>> {
        StreamableHttpClient::post_message(
            &self.client,
            uri,
            message,
            session_id,
            auth_header,
            custom_headers,
        )
        .await
    }

    async fn post_message_with_max_sse_event_size(
        &self,
        uri: Arc<str>,
        message: ClientJsonRpcMessage,
        session_id: Option<Arc<str>>,
        auth_header: Option<String>,
        custom_headers: HashMap<HeaderName, HeaderValue>,
        max_sse_event_size: usize,
    ) -> Result<StreamableHttpPostResponse, StreamableHttpError<Self::Error>> {
        StreamableHttpClient::post_message_with_max_sse_event_size(
            &self.client,
            uri,
            message,
            session_id,
            auth_header,
            custom_headers,
            max_sse_event_size,
        )
        .await
    }

    async fn delete_session(
        &self,
        uri: Arc<str>,
        session_id: Arc<str>,
        auth_header: Option<String>,
        custom_headers: HashMap<HeaderName, HeaderValue>,
    ) -> Result<(), StreamableHttpError<Self::Error>> {
        StreamableHttpClient::delete_session(&self.client, uri, session_id, auth_header, custom_headers)
            .await
    }

    async fn get_stream(
        &self,
        uri: Arc<str>,
        session_id: Option<Arc<str>>,
        last_event_id: Option<String>,
        auth_header: Option<String>,
        custom_headers: HashMap<HeaderName, HeaderValue>,
    ) -> Result<BoxStream<'static, Result<Sse, SseError>>, StreamableHttpError<Self::Error>> {
        StreamableHttpClient::get_stream(
            &self.client,
            uri,
            session_id,
            last_event_id,
            auth_header,
            custom_headers,
        )
        .await
    }

    async fn get_stream_with_max_sse_event_size(
        &self,
        uri: Arc<str>,
        session_id: Option<Arc<str>>,
        last_event_id: Option<String>,
        auth_header: Option<String>,
        custom_headers: HashMap<HeaderName, HeaderValue>,
        max_sse_event_size: usize,
    ) -> Result<BoxStream<'static, Result<Sse, SseError>>, StreamableHttpError<Self::Error>> {
        StreamableHttpClient::get_stream_with_max_sse_event_size(
            &self.client,
            uri,
            session_id,
            last_event_id,
            auth_header,
            custom_headers,
            max_sse_event_size,
        )
        .await
    }
}
