package com.lulu.workbench.android.agent.tools

import com.lulu.workbench.android.agent.tools.fs.FsTools
import com.lulu.workbench.android.agent.tools.stage.StageTools
import com.lulu.workbench.android.llm.LlmToolDef
import com.lulu.workbench.android.storage.Storage
import com.lulu.workbench.android.wmcp.WmcpClient

class ToolDispatcher(
    private val fs: FsTools,
    private val wmcp: WmcpClient,
    private val storage: Storage,
) {
    private val stage = StageTools(storage)

    fun definitions(): List<LlmToolDef> {
        val local = fs.definitions() + stage.definitions()
        if (!wmcp.isBound()) return local
        val remote = try {
            wmcp.listTools()
        } catch (_: Exception) {
            emptyList()
        }
        return local +
            remote.map { tool ->
                val description =
                    if (tool.name == NOTE_CONTENT_TOOL) {
                        NOTE_CONTENT_STAGE_DESCRIPTION
                    } else {
                        tool.description.ifEmpty { tool.name }
                    }
                LlmToolDef(
                    name = tool.name,
                    description = description,
                    parametersJson = tool.inputSchemaJson.ifBlank { """{"type":"object"}""" },
                )
            }
    }

    fun call(
        name: String,
        arguments: String,
        toolRoot: String,
        sessionId: String = "",
        sessionTitle: String = "",
    ): String {
        if (name in fs.names) {
            return fs.call(name, arguments, toolRoot, storage)
        }
        if (name in stage.names) {
            return stage.call(name, arguments, sessionId, sessionTitle)
        }
        if (!wmcp.isBound()) {
            return "Tool '$name' is not available."
        }
        return try {
            val remoteText = wmcp.callTool(name, arguments).text
            if (name == NOTE_CONTENT_TOOL) {
                stageNoteContentResult(remoteText, stage, sessionId, sessionTitle)
            } else {
                remoteText
            }
        } catch (error: Exception) {
            error.message ?: "Tool '$name' failed."
        }
    }
}
