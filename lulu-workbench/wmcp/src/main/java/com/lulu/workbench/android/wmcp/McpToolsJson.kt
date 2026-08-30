package com.lulu.workbench.android.wmcp

private const val EMPTY_OBJECT_SCHEMA = """{"type":"object"}"""

internal fun parseToolNames(body: String): List<String> = parseTools(body).map { it.name }

internal fun parseTools(body: String): List<McpTool> {
    val toolsAt = body.indexOf("\"tools\"")
    if (toolsAt < 0) return emptyList()
    val arrayAt = body.indexOf('[', startIndex = toolsAt)
    if (arrayAt < 0) return emptyList()
    val arrayEnd = matchingCloser(body, arrayAt)
    if (arrayEnd < 0) return emptyList()
    val out = mutableListOf<McpTool>()
    var cursor = arrayAt + 1
    while (cursor < arrayEnd) {
        val objAt = nextUnquoted(body, '{', cursor, arrayEnd)
        if (objAt < 0) return out
        val objEnd = matchingCloser(body, objAt)
        if (objEnd < 0 || objEnd > arrayEnd) return out
        val obj = body.substring(objAt, objEnd + 1)
        val name = topLevelString(obj, "name")
        if (name.isNotEmpty()) {
            val schema =
                topLevelObject(obj, "inputSchema").ifEmpty { topLevelObject(obj, "input_schema") }
            out.add(
                McpTool(
                    name = name,
                    description = topLevelString(obj, "description"),
                    inputSchemaJson = schema.ifEmpty { EMPTY_OBJECT_SCHEMA },
                ),
            )
        }
        cursor = objEnd + 1
    }
    return out
}

private fun topLevelString(obj: String, key: String): String {
    val colon = topLevelColon(obj, key) ?: return ""
    val start = skipWs(obj, colon + 1)
    if (start >= obj.length || obj[start] != '"') return ""
    return readJsonString(obj, start)
}

private fun topLevelObject(obj: String, key: String): String {
    val colon = topLevelColon(obj, key) ?: return ""
    val start = skipWs(obj, colon + 1)
    if (start >= obj.length || obj[start] != '{') return ""
    val end = matchingCloser(obj, start)
    if (end < 0) return ""
    return obj.substring(start, end + 1)
}

private fun topLevelColon(obj: String, key: String): Int? {
    val needle = "\"$key\""
    var i = 1
    var depth = 1
    var inString = false
    while (i < obj.length && depth > 0) {
        val c = obj[i]
        if (inString) {
            if (c == '\\' && i + 1 < obj.length) {
                i += 2
                continue
            }
            if (c == '"') inString = false
            i += 1
            continue
        }
        when (c) {
            '"' -> {
                if (depth == 1 && obj.startsWith(needle, i)) {
                    return obj.indexOf(':', startIndex = i + needle.length).takeIf { it >= 0 }
                }
                inString = true
            }
            '{', '[' -> depth += 1
            '}', ']' -> depth -= 1
        }
        i += 1
    }
    return null
}

private fun nextUnquoted(json: String, needle: Char, from: Int, until: Int): Int {
    var i = from
    var inString = false
    while (i < until) {
        val c = json[i]
        if (inString) {
            if (c == '\\' && i + 1 < until) {
                i += 2
                continue
            }
            if (c == '"') inString = false
            i += 1
            continue
        }
        if (c == '"') {
            inString = true
        } else if (c == needle) {
            return i
        }
        i += 1
    }
    return -1
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
            else -> out.append(next)
        }
        i += 2
    }
    return out.toString()
}

private fun skipWs(json: String, from: Int): Int {
    var i = from
    while (i < json.length && json[i].isWhitespace()) i += 1
    return i
}
