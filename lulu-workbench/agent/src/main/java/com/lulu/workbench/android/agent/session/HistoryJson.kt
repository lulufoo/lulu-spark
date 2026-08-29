package com.lulu.workbench.android.agent.session

data class HistoryTurn(
    val role: String,
    val content: String,
)

internal fun encodeTurns(turns: List<HistoryTurn>): String {
    val items = turns.joinToString(",") { turn ->
        """{"role":"${escape(turn.role)}","content":"${escape(turn.content)}"}"""
    }
    return "[$items]"
}

internal fun decodeTurns(json: String): List<HistoryTurn> {
    if (json.isBlank()) return emptyList()
    val turns = mutableListOf<HistoryTurn>()
    var cursor = 0
    while (true) {
        val roleKey = json.indexOf("\"role\"", startIndex = cursor)
        if (roleKey < 0) return turns
        val role = readJsonStringAfterKey(json, roleKey)
        val contentKey = json.indexOf("\"content\"", startIndex = roleKey)
        if (contentKey < 0) return turns
        val content = readJsonStringAfterKey(json, contentKey)
        turns.add(HistoryTurn(role = role, content = content))
        cursor = contentKey + 9
    }
}

private fun readJsonStringAfterKey(json: String, keyAt: Int): String {
    val colon = json.indexOf(':', startIndex = keyAt)
    val quote = json.indexOf('"', startIndex = colon + 1)
    return readJsonString(json, quote)
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
