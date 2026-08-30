package com.lulu.workbench.android.agent.tools

internal fun jsonStringField(json: String, key: String): String? = jsonRawField(json, key)

internal fun jsonIntField(json: String, key: String): Int? = jsonRawField(json, key)?.toIntOrNull()

internal fun jsonBoolField(json: String, key: String): Boolean? =
    when (jsonRawField(json, key)?.trim()) {
        "true" -> true
        "false" -> false
        else -> null
    }

private fun jsonRawField(json: String, key: String): String? {
    val needle = "\"$key\""
    val at = json.indexOf(needle)
    if (at < 0) return null
    val colon = json.indexOf(':', startIndex = at + needle.length)
    if (colon < 0) return null
    var i = colon + 1
    while (i < json.length && json[i].isWhitespace()) i += 1
    if (i >= json.length) return null
    if (json[i] == '"') return readJsonString(json, i)
    val end = json.indexOfFirstFrom(i) { ch -> ch == ',' || ch == '}' || ch.isWhitespace() }
    return json.substring(i, end)
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

private fun String.indexOfFirstFrom(start: Int, predicate: (Char) -> Boolean): Int {
    var i = start
    while (i < length && !predicate(this[i])) i += 1
    return i
}
