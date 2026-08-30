package com.lulu.workbench.android.agent.tools.stage

import com.lulu.workbench.android.agent.tools.jsonBoolField
import com.lulu.workbench.android.agent.tools.jsonStringField
import com.lulu.workbench.android.llm.LlmToolDef
import com.lulu.workbench.android.storage.Storage

class StageTools(
    private val store: StageStore,
) {
    constructor(storage: Storage) : this(StageStore(storage))

    val names: List<String> = listOf("stage", "list_staged", "get_staged")

    fun definitions(): List<LlmToolDef> =
        listOf(
            LlmToolDef(
                "stage",
                "Save a Markdown document to the on-device staged library. This is not a Workbench note. Use create_note later when MCP is connected and the user asks.",
                STAGE_PARAMS,
            ),
            LlmToolDef(
                "list_staged",
                "List staged Markdown documents (handle F1, id, title, source chat). Does not return bodies.",
                LIST_PARAMS,
            ),
            LlmToolDef(
                "get_staged",
                "Read one staged Markdown document by handle (F1) or id.",
                GET_PARAMS,
            ),
        )

    fun createFromBody(
        title: String,
        content: String,
        sessionId: String,
        sessionTitle: String,
    ) = store.create(title, content, sessionId, sessionTitle)

    fun call(
        name: String,
        arguments: String,
        sessionId: String,
        sessionTitle: String,
    ): String =
        try {
            when (name) {
                "stage" -> stage(arguments, sessionId, sessionTitle)
                "list_staged" -> list(arguments, sessionId)
                "get_staged" -> get(arguments)
                else -> "unknown stage tool '$name'"
            }
        } catch (error: IllegalArgumentException) {
            error.message ?: "invalid arguments"
        }

    private fun stage(arguments: String, sessionId: String, sessionTitle: String): String {
        val content = jsonStringField(arguments, "content") ?: return "missing content"
        val title = jsonStringField(arguments, "title").orEmpty()
        val item = store.create(title, content, sessionId, sessionTitle)
        return "Staged ${item.handle} title=${item.title} id=${item.id}"
    }

    private fun list(arguments: String, sessionId: String): String {
        val onlyThis = jsonBoolField(arguments, "this_chat_only") == true
        val items = if (onlyThis) store.listForSession(sessionId) else store.list()
        if (items.isEmpty()) return "No staged files."
        return items.joinToString("\n") { item ->
            val here = item.sourceSessionId == sessionId
            "handle=${item.handle} id=${item.id} this_chat=$here title=${item.title} source=${item.sourceSessionTitle}"
        }
    }

    private fun get(arguments: String): String {
        val id = jsonStringField(arguments, "id") ?: return "missing id"
        val item = store.get(id) ?: return "staged file not found"
        return buildString {
            appendLine("handle: ${item.handle}")
            appendLine("id: ${item.id}")
            appendLine("title: ${item.title}")
            appendLine("source_session_id: ${item.sourceSessionId}")
            appendLine("source_session_title: ${item.sourceSessionTitle}")
            appendLine()
            append(item.body)
        }.trimEnd()
    }
}

private const val STAGE_PARAMS =
    """{"type":"object","properties":{"title":{"type":"string"},"content":{"type":"string"}},"required":["content"]}"""
private const val LIST_PARAMS =
    """{"type":"object","properties":{"this_chat_only":{"type":"boolean"}}}"""
private const val GET_PARAMS =
    """{"type":"object","properties":{"id":{"type":"string"}},"required":["id"]}"""
