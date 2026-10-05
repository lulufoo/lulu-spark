package com.lulu.spark.android.agent.tools

import com.lulu.spark.android.agent.tools.stage.StageTools

internal const val NOTE_CONTENT_TOOL = "get_note_content_by_id"

internal const val NOTE_CONTENT_STAGE_DESCRIPTION =
    "Fetch one note's raw Markdown onto the phone staged library and tag this chat. On success returns success plus a staged handle. Does not return the body. The user reads it in Stage. Use get_note_digest_by_id for a digest, or get_staged for the staged file."

internal fun stageNoteContentResult(
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
    val noteId = jsonStringField(inner, "id").orEmpty()
    val title = titleFromNoteRaw(content, noteId)
    val item = stage.createFromBody(title, content, sessionId, sessionTitle)
    return "success handle=${item.handle} id=${item.id} note_id=$noteId"
}

internal fun titleFromNoteRaw(body: String, noteId: String): String {
    for (line in body.lineSequence()) {
        val trimmed = line.trim()
        if (trimmed.startsWith("# ")) {
            val heading = trimmed.removePrefix("# ").trim()
            if (heading.isNotEmpty()) return heading
        }
    }
    return noteId.ifBlank { "Untitled" }
}
