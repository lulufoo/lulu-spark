package com.lulu.workbench.android.agent.tools

import com.lulu.workbench.android.agent.tools.fs.FsTools
import com.lulu.workbench.android.llm.LlmToolDef
import com.lulu.workbench.android.storage.Storage
import com.lulu.workbench.android.wmcp.WmcpClient

class ToolDispatcher(
    private val fs: FsTools,
    private val wmcp: WmcpClient,
    private val storage: Storage,
) {
    fun definitions(): List<LlmToolDef> {
        val local = fs.definitions()
        if (!wmcp.isBound()) return local
        val remote = try {
            wmcp.listTools()
        } catch (_: Exception) {
            emptyList()
        }
        return local +
            remote.map { tool ->
                LlmToolDef(
                    name = tool.name,
                    description = tool.description.ifEmpty { tool.name },
                    parametersJson = tool.inputSchemaJson.ifBlank { """{"type":"object"}""" },
                )
            }
    }

    fun call(name: String, arguments: String, toolRoot: String): String {
        if (name in fs.names) {
            return fs.call(name, arguments, toolRoot, storage)
        }
        if (!wmcp.isBound()) {
            return "Tool '$name' is not available."
        }
        return try {
            wmcp.callTool(name, arguments).text
        } catch (error: Exception) {
            error.message ?: "Tool '$name' failed."
        }
    }
}
