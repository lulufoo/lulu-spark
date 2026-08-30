package com.lulu.workbench.android.wmcp.mcp

import com.lulu.workbench.android.wmcp.McpFailedException
import com.lulu.workbench.android.wmcp.shared.jsonString

internal const val MCP_PROTOCOL_VERSION = "2025-03-26"
internal const val MCP_ACCEPT = "application/json, text/event-stream"

internal fun mcpInitializeBody(id: Int): String =
    """{"jsonrpc":"2.0","id":$id,"method":"initialize","params":{"protocolVersion":"$MCP_PROTOCOL_VERSION","capabilities":{},"clientInfo":{"name":"workbench-android","version":"0.1.0"}}}"""

internal fun mcpInitializedBody(): String =
    """{"jsonrpc":"2.0","method":"notifications/initialized","params":{}}"""

internal fun mcpRequestBody(id: Int, method: String, params: String): String =
    """{"jsonrpc":"2.0","id":$id,"method":"$method","params":$params}"""

internal fun headerValue(headers: Map<String, String>, name: String): String? {
    val needle = name.lowercase()
    return headers.entries.firstOrNull { it.key.equals(needle, ignoreCase = true) }?.value
}

internal fun initializeAccepted(raw: String) {
    val payload = jsonRpcPayload(raw)
    if (jsonRpcHasError(payload)) {
        throw McpFailedException("mcp initialize rejected")
    }
    val version = optionalJsonString(payload, "protocolVersion")
    if (version.isNotEmpty() && version != MCP_PROTOCOL_VERSION) {
        throw McpFailedException("mcp protocol $version")
    }
}

internal fun jsonRpcHasError(payload: String): Boolean {
    val errorAt = payload.indexOf("\"error\"")
    if (errorAt < 0) return false
    val resultAt = payload.indexOf("\"result\"")
    return resultAt < 0 || errorAt < resultAt
}

internal fun optionalJsonString(json: String, key: String): String =
    try {
        jsonString(json, key)
    } catch (_: IllegalArgumentException) {
        ""
    }

internal fun jsonRpcPayload(raw: String): String {
    val chunks = raw.lineSequence()
        .map { it.trimEnd() }
        .filter { it.startsWith("data:") }
        .map { it.removePrefix("data:").trim() }
        .filter { it.isNotEmpty() && it != "[DONE]" }
        .toList()
    return chunks.lastOrNull { it.startsWith("{") } ?: raw
}
