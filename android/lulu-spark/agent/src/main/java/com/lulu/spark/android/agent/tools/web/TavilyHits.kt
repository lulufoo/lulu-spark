package com.lulu.spark.android.agent.tools.web

import com.lulu.spark.android.agent.tools.jsonStringField

internal data class WebHit(
    val title: String,
    val url: String,
    val snippet: String,
)

internal fun parseTavilyHits(json: String, snippetMax: Int = SNIPPET_MAX): List<WebHit> {
    val key = json.indexOf("\"results\"")
    if (key < 0) return emptyList()
    val start = json.indexOf('[', startIndex = key)
    if (start < 0) return emptyList()
    val end = matchingCloser(json, start)
    if (end < 0) return emptyList()
    val hits = mutableListOf<WebHit>()
    var cursor = start + 1
    while (cursor < end && hits.size < MAX_LIMIT) {
        val objAt = json.indexOf('{', startIndex = cursor)
        if (objAt < 0 || objAt >= end) return hits
        val objEnd = matchingCloser(json, objAt)
        if (objEnd < 0 || objEnd > end) return hits
        val obj = json.substring(objAt, objEnd + 1)
        val url = jsonStringField(obj, "url").orEmpty()
        if (url.isNotBlank()) {
            hits.add(
                WebHit(
                    title = jsonStringField(obj, "title").orEmpty(),
                    url = url,
                    snippet = clipSnippet(jsonStringField(obj, "content").orEmpty(), snippetMax),
                ),
            )
        }
        cursor = objEnd + 1
    }
    return hits
}

private fun clipSnippet(raw: String, max: Int): String {
    val text = raw.replace('\n', ' ').trim()
    if (text.length <= max) return text
    return text.take(max).trimEnd() + "…"
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

internal const val SNIPPET_MAX = 280
