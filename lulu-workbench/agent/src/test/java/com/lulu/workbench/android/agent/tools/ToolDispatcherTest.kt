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

    @Test
    fun noteContentDescriptionSaysStagedNotBody() {
        val defs =
            ToolDispatcher(
                FsTools(),
                BoundWmcp(
                    McpTool(
                        name = "get_note_content_by_id",
                        description = "Read one note's raw Markdown by archive entry id.",
                        inputSchemaJson = """{"type":"object","properties":{"id":{"type":"string"}},"required":["id"]}""",
                    ),
                ),
                MemoryStorage(),
            ).definitions()
        val tool = defs.single { it.name == "get_note_content_by_id" }
        assertEquals(NOTE_CONTENT_STAGE_DESCRIPTION, tool.description)
        assertTrue(tool.parametersJson.contains("\"id\""))
    }

    @Test
    fun noteContentSuccessStagesAndHidesBody() {
        val storage = MemoryStorage()
        val noteId = "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee"
        val host =
            """{"id":"$noteId","ok":true,"content":"# Park note\n\nsecret body","truncated":false}"""
        val dispatcher =
            ToolDispatcher(
                FsTools(),
                CallingWmcp("get_note_content_by_id", host),
                storage,
            )
        val out =
            dispatcher.call(
                "get_note_content_by_id",
                """{"id":"$noteId"}""",
                "/tmp",
                "sess_a",
                "commute chat",
            )
        assertTrue(out.startsWith("success handle=F1 "))
        assertTrue(out.contains("note_id=$noteId"))
        assertTrue(!out.contains("secret body"))
        val body = storage.list("staged").single { it.endsWith("/body.md") }
        assertEquals("# Park note\n\nsecret body", storage.read(body)?.decodeToString())
        val meta = storage.read(body.removeSuffix("/body.md") + "/meta")?.decodeToString().orEmpty()
        assertTrue(meta.contains("\"source_session_id\":\"sess_a\""))
        assertTrue(meta.contains("\"source_session_title\":\"commute chat\""))
        assertTrue(meta.contains("Park note"))
    }

    @Test
    fun noteContentFailureDoesNotStage() {
        val storage = MemoryStorage()
        val dispatcher =
            ToolDispatcher(
                FsTools(),
                CallingWmcp(
                    "get_note_content_by_id",
                    """{"id":"ffffffffffffffffffffffffffffffff","ok":false,"error":"Entry not found"}""",
                ),
                storage,
            )
        val out =
            dispatcher.call(
                "get_note_content_by_id",
                """{"id":"ffffffffffffffffffffffffffffffff"}""",
                "/tmp",
                "sess_a",
                "chat",
            )
        assertEquals("error: Entry not found", out)
        assertTrue(storage.list("staged").isEmpty())
    }

    @Test
    fun otherRemoteToolsPassThrough() {
        val dispatcher =
            ToolDispatcher(
                FsTools(),
                CallingWmcp("list_todo_tasks", """[{"title":"keep"}]"""),
                MemoryStorage(),
            )
        assertEquals(
            """[{"title":"keep"}]""",
            dispatcher.call("list_todo_tasks", "{}", "/tmp"),
        )
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

private class CallingWmcp(
    private val expectedName: String,
    private val result: String,
) : WmcpClient {
    override fun isBound(): Boolean = true

    override fun deviceId(): String = "dev_call"

    override fun completeBind(offer: BindOffer, deviceLabel: String?): BindResult = error("unused")

    override fun listTools(): List<McpTool> = emptyList()

    override fun callTool(name: String, arguments: String): McpToolResult {
        if (name != expectedName) error("unexpected tool $name")
        return McpToolResult(text = result)
    }
}

private class UnboundForDispatch : WmcpClient {
    override fun isBound(): Boolean = false

    override fun deviceId(): String = "dev_unbound"

    override fun completeBind(offer: BindOffer, deviceLabel: String?): BindResult = error("unused")

    override fun listTools(): List<McpTool> = emptyList()

    override fun callTool(name: String, arguments: String): McpToolResult = error("unused")
}
