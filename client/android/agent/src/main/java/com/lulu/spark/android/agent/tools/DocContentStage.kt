package com.lulu.spark.android.agent.tools

import com.lulu.spark.android.agent.tools.stage.StageTools

internal const val DOC_CONTENT_TOOL = "get_doc_content"

internal const val DOC_CONTENT_STAGE_DESCRIPTION =
    "Fetch one document's raw Markdown onto the phone staged library and tag this chat. Pass the workspace the search hit came from. On success returns success plus a staged handle. Does not return the body. The user reads it in Stage. Use get_doc_digest for a digest, or get_staged for the staged file."

internal fun stageDocContentResult(
    remoteText: String,
    stage: StageTools,
    sessionId: String,
    sessionTitle: String,
): String {
    if (jsonBoolField(remoteText, "isError") == true) {
        return jsonStringField(remoteText, "text") ?: remoteText
    }
    val inner = jsonStringField(remoteText, "text") ?: remoteText
    if (jsonBoolField(inner, "ok") == false) {
        return jsonStringField(inner, "error")?.let { "error: $it" } ?: inner
    }
    val content = jsonStringField(inner, "content") ?: return inner
    val docId = jsonStringField(inner, "id").orEmpty()
    val title = titleFromDocRaw(content, docId)
    val item = stage.createFromBody(title, content, sessionId, sessionTitle)
    return "success handle=${item.handle} id=${item.id} doc_id=$docId"
}

internal fun titleFromDocRaw(body: String, docId: String): String {
    for (line in body.lineSequence()) {
        val trimmed = line.trim()
        if (trimmed.startsWith("# ")) {
            val heading = trimmed.removePrefix("# ").trim()
            if (heading.isNotEmpty()) return heading
        }
    }
    return docId.ifBlank { "Untitled" }
}
