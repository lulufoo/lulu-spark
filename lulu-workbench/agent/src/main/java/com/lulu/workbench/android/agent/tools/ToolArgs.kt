package com.lulu.workbench.android.agent.tools

internal fun jsonStringField(json: String, key: String): String? {
    val raw = jsonRawField(json, key) ?: return null
    return if (raw.startsWith("\"")) unescape(readQuoted(raw)) else raw
}

internal fun jsonIntField(json: String, key: String): Int? = jsonRawField(json, key)?.toIntOrNull()

private fun jsonRawField(json: String, key: String): String? {
    val needle = "\"$key\""
    val at = json.indexOf(needle)
    if (at < 0) return null
    val colon = json.indexOf(':', startIndex = at + needle.length)
    if (colon < 0) return null
    var i = colon + 1
    while (i < json.length && json[i].isWhitespace()) i += 1
    if (i >= json.length) return null
    if (json[i] == '"') {
        return "\"" + readJsonString(json, i) + "\""
    }
    val end = json.indexOfFirstFrom(i) { ch -> ch == ',' || ch == '}' || ch.isWhitespace() }
    return json.substring(i, end)
}

private fun readQuoted(quoted: String): String = readJsonString(quoted, 0)

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

private fun unescape(value: String): String =
    value.replace("\\n", "\n").replace("\\\"", "\"").replace("\\\\", "\\")

private fun String.indexOfFirstFrom(start: Int, predicate: (Char) -> Boolean): Int {
    var i = start
    while (i < length && !predicate(this[i])) i += 1
    return i
}
