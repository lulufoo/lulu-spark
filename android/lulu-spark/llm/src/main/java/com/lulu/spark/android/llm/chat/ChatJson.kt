package com.lulu.spark.android.llm.chat

import com.lulu.spark.android.llm.LlmMessage
import com.lulu.spark.android.llm.LlmToolCall
import com.lulu.spark.android.llm.LlmToolDef

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
    return readJsonString(from, quote)
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
    if (start < 0) return emptyList()
    val end = matchingCloser(json, start)
    if (end < 0) return emptyList()
    val calls = mutableListOf<LlmToolCall>()
    var cursor = start + 1
    while (cursor < end) {
        val objAt = json.indexOf('{', startIndex = cursor)
        if (objAt < 0 || objAt >= end) return calls
        val objEnd = matchingCloser(json, objAt)
        if (objEnd < 0 || objEnd > end) return calls
        val obj = json.substring(objAt, objEnd + 1)
        val idAt = obj.indexOf("\"id\"")
        val nameAt = obj.indexOf("\"name\"")
        val argsAt = obj.indexOf("\"arguments\"")
        if (idAt >= 0 && nameAt >= 0) {
            val id = readStringAfterKey(obj, idAt)
            val name = readStringAfterKey(obj, nameAt)
            val arguments = if (argsAt >= 0) readJsonAfterKey(obj, argsAt) else "{}"
            if (id.isNotEmpty() && name.isNotEmpty()) {
                calls.add(LlmToolCall(id = id, name = name, arguments = arguments))
            }
        }
        cursor = objEnd + 1
    }
    return calls
}

internal fun toolCallsPreview(json: String): String {
    val key = json.indexOf("\"tool_calls\"")
    if (key < 0) return "-"
    val start = json.indexOf('[', startIndex = key)
    if (start < 0) return "no-array"
    val end = matchingCloser(json, start)
    val raw = if (end < 0) json.substring(start).take(160) else json.substring(start, end + 1)
    return if (raw.length > 160) raw.take(160) + "…" else raw
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
    return readJsonString(json, quote)
}

private fun readJsonAfterKey(json: String, keyAt: Int): String {
    val colon = json.indexOf(':', startIndex = keyAt)
    if (colon < 0) return ""
    var i = colon + 1
    while (i < json.length && json[i].isWhitespace()) i += 1
    if (i >= json.length) return ""
    return when (json[i]) {
        '"' -> readJsonString(json, i)
        '{', '[' -> {
            val close = matchingCloser(json, i)
            if (close < 0) "" else json.substring(i, close + 1)
        }
        else -> ""
    }
}

private fun matchingCloser(json: String, openAt: Int): Int {
    val open = json[openAt]
    val close = if (open == '[') ']' else '}'
    var depth = 0
    var i = openAt
    var inString = false
    while (i < json.length) {
        val c = json[i]
        if (inString) {
            if (c == '\\' && i + 1 < json.length) {
                i += 2
                continue
            }
            if (c == '"') inString = false
            i += 1
            continue
        }
        when (c) {
            '"' -> inString = true
            open -> depth += 1
            close -> {
                depth -= 1
                if (depth == 0) return i
            }
        }
        i += 1
    }
    return -1
}

private fun readJsonString(source: String, openQuote: Int): String {
    val out = StringBuilder()
    var i = openQuote + 1
    while (i < source.length) {
        val c = source[i]
        if (c == '"') break
        if (c != '\\' || i + 1 >= source.length) {
            out.append(c)
            i += 1
            continue
        }
        when (val next = source[i + 1]) {
            'n' -> out.append('\n')
            'r' -> out.append('\r')
            't' -> out.append('\t')
            '"' -> out.append('"')
            '\\' -> out.append('\\')
            '/' -> out.append('/')
            'u' -> {
                if (i + 5 < source.length) {
                    val code = source.substring(i + 2, i + 6).toIntOrNull(16)
                    if (code != null) {
                        out.append(code.toChar())
                        i += 6
                        continue
                    }
                }
                out.append('u')
            }
            else -> out.append(next)
        }
        i += 2
    }
    return out.toString()
}

private fun escape(value: String): String =
    value.replace("\\", "\\\\").replace("\"", "\\\"").replace("\n", "\\n")
