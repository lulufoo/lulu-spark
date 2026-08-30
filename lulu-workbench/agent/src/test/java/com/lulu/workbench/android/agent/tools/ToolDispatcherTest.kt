package com.lulu.workbench.android.agent.tools

import com.lulu.workbench.android.agent.tools.fs.FsTools
import com.lulu.workbench.android.storage.MemoryStorage
import com.lulu.workbench.android.wmcp.BindOffer
import com.lulu.workbench.android.wmcp.BindResult
import com.lulu.workbench.android.wmcp.McpTool
import com.lulu.workbench.android.wmcp.McpToolResult
import com.lulu.workbench.android.wmcp.WmcpClient
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class ToolDispatcherTest {
    @Test
    fun remoteToolKeepsHostInputSchema() {
        val schema =
            """{"type":"object","properties":{"title":{"type":"string"},"content":{"type":"string"}},"required":["title","content"]}"""
        val defs =
            ToolDispatcher(
                FsTools(),
                BoundWmcp(
                    McpTool(
                        name = "create_note",
                        description = "Create a note from Markdown content.",
                        inputSchemaJson = schema,
                    ),
                ),
                MemoryStorage(),
            ).definitions()
        val create = defs.single { it.name == "create_note" }
        assertEquals("Create a note from Markdown content.", create.description)
        assertEquals(schema, create.parametersJson)
        assertTrue(create.parametersJson.contains("\"title\""))
        assertTrue(create.parametersJson.contains("\"content\""))
    }

    @Test
    fun unboundLocalToolsIncludeFsAndStage() {
        val defs =
            ToolDispatcher(FsTools(), UnboundForDispatch(), MemoryStorage()).definitions()
        val names = defs.map { it.name }
        assertTrue(names.containsAll(listOf("read", "grep", "write", "edit")))
        assertTrue(names.containsAll(listOf("stage", "list_staged", "get_staged")))
        assertTrue(defs.none { it.name == "create_note" })
        assertTrue(defs.none { it.name == "delete_staged" })
    }
}

private class BoundWmcp(
    private vararg val tools: McpTool,
) : WmcpClient {
    override fun isBound(): Boolean = true

    override fun deviceId(): String = "dev_bound"

    override fun completeBind(offer: BindOffer, deviceLabel: String?): BindResult = error("unused")

    override fun listTools(): List<McpTool> = tools.toList()

    override fun callTool(name: String, arguments: String): McpToolResult = error("unused")
}

private class UnboundForDispatch : WmcpClient {
    override fun isBound(): Boolean = false

    override fun deviceId(): String = "dev_unbound"

    override fun completeBind(offer: BindOffer, deviceLabel: String?): BindResult = error("unused")

    override fun listTools(): List<McpTool> = emptyList()

    override fun callTool(name: String, arguments: String): McpToolResult = error("unused")
}
