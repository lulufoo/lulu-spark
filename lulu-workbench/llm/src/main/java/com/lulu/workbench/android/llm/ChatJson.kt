package com.lulu.workbench.android.llm

internal fun chatUrl(baseUrl: String): String {
    val base = baseUrl.trim().trimEnd('/')
    return if (base.endsWith("/v1") || base.endsWith("/v4")) {
        "$base/chat/completions"
    } else {
        "$base/v1/chat/completions"
    }
}

internal fun encodeChatBody(
    model: String,
    messages: List<LlmMessage>,
    tools: List<LlmToolDef> = emptyList(),
): String {
    val items = messages.joinToString(",") { encodeMessage(it) }
    val toolPart =
        if (tools.isEmpty()) {
            ""
        } else {
            val defs =
                tools.joinToString(",") { tool ->
                    """{"type":"function","function":{"name":"${escape(tool.name)}","description":"${escape(tool.description)}","parameters":${tool.parametersJson}}}"""
                }
            ""","tools":[$defs]"""
        }
    return """{"model":"${escape(model)}","stream":false,"messages":[$items]$toolPart}"""
}

internal fun decodeAssistantText(json: String): String {
    val messageAt = json.indexOf("\"message\"")
    val from = if (messageAt >= 0) json.substring(messageAt) else json
    val key = from.indexOf("\"content\"")
    if (key < 0) return ""
    val colon = from.indexOf(':', startIndex = key)
    val quote = from.indexOf('"', startIndex = colon + 1)
    if (quote < 0) return ""
    return unescape(readJsonString(from, quote))
}

internal fun decodeFinishReason(json: String): String {
    val key = json.indexOf("\"finish_reason\"")
    if (key < 0) return ""
    return readStringAfterKey(json, key)
}

internal fun hasToolCallsKey(json: String): Boolean = json.indexOf("\"tool_calls\"") >= 0

internal fun decodeToolCalls(json: String): List<LlmToolCall> {
    val key = json.indexOf("\"tool_calls\"")
    if (key < 0) return emptyList()
    val start = json.indexOf('[', startIndex = key)
    val end = json.indexOf(']', startIndex = start)
    if (start < 0 || end < 0) return emptyList()
    val slice = json.substring(start, end + 1)
    val calls = mutableListOf<LlmToolCall>()
    var cursor = 0
    while (true) {
        val idAt = slice.indexOf("\"id\"", startIndex = cursor)
        if (idAt < 0) return calls
        val nameAt = slice.indexOf("\"name\"", startIndex = idAt)
        val argsAt = slice.indexOf("\"arguments\"", startIndex = idAt)
        if (nameAt < 0 || argsAt < 0) return calls
        val id = readStringAfterKey(slice, idAt)
        val name = readStringAfterKey(slice, nameAt)
        val arguments = readStringAfterKey(slice, argsAt)
        if (id.isNotEmpty() && name.isNotEmpty()) {
            calls.add(LlmToolCall(id = id, name = name, arguments = arguments))
        }
        cursor = maxOf(nameAt, argsAt) + 1
    }
}

private fun encodeMessage(message: LlmMessage): String {
    val parts = mutableListOf(
        """"role":"${escape(message.role)}"""",
        """"content":"${escape(message.content)}"""",
    )
    val toolCallId = message.toolCallId
    if (toolCallId != null) {
        parts.add(""""tool_call_id":"${escape(toolCallId)}"""")
    }
    if (message.toolCalls.isNotEmpty()) {
        val calls =
            message.toolCalls.joinToString(",") { call ->
                """{"id":"${escape(call.id)}","type":"function","function":{"name":"${escape(call.name)}","arguments":"${escape(call.arguments)}"}}"""
            }
        parts.add(""""tool_calls":[$calls]""")
    }
    return "{${parts.joinToString(",")}}"
}

private fun readStringAfterKey(json: String, keyAt: Int): String {
    val colon = json.indexOf(':', startIndex = keyAt)
    val quote = json.indexOf('"', startIndex = colon + 1)
    if (quote < 0) return ""
    return unescape(readJsonString(json, quote))
}

private fun readJsonString(source: String, openQuote: Int): String {
    val out = StringBuilder()
    var i = openQuote + 1
    while (i < source.length) {
        val c = source[i]
        if (c == '\\' && i + 1 < source.length) {
            out.append(source[i + 1])
            i += 2
            continue
        }
        if (c == '"') break
        out.append(c)
        i += 1
    }
    return out.toString()
}

private fun escape(value: String): String =
    value.replace("\\", "\\\\").replace("\"", "\\\"").replace("\n", "\\n")

private fun unescape(value: String): String =
    value.replace("\\n", "\n").replace("\\\"", "\"").replace("\\\\", "\\")
