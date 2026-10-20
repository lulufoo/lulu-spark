package com.lulu.spark.android.agent.tools

import com.lulu.spark.android.agent.tools.fs.FsTools
import com.lulu.spark.android.agent.tools.web.SECRET_NAME
import com.lulu.spark.android.agent.tools.web.WebSearchTools
import com.lulu.spark.android.network.HttpRequest
import com.lulu.spark.android.network.HttpResponse
import com.lulu.spark.android.network.NetworkClient
import com.lulu.spark.android.storage.MemoryStorage
import com.lulu.spark.android.wmcp.BindOffer
import com.lulu.spark.android.wmcp.BindResult
import com.lulu.spark.android.wmcp.McpTool
import com.lulu.spark.android.wmcp.McpToolResult
import com.lulu.spark.android.wmcp.WmcpClient
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class ToolDispatcherTest {
    @Test
    fun remoteToolKeepsHostInputSchema() {
        val schema =
            """{"type":"object","properties":{"title":{"type":"string"},"content":{"type":"string"},"workspace_id":{"type":"string"},"project_id":{"type":"string"},"folder":{"type":"string"}},"required":["title","content","workspace_id"]}"""
        val defs =
            ToolDispatcher(
                FsTools(),
                BoundWmcp(
                    McpTool(
                        name = "create_doc",
                        description = "Create a document from Markdown content.",
                        inputSchemaJson = schema,
                    ),
                ),
                MemoryStorage(),
            ).definitions()
        val create = defs.single { it.name == "create_doc" }
        assertEquals("Create a document from Markdown content.", create.description)
        assertEquals(schema, create.parametersJson)
        assertTrue(create.parametersJson.contains("\"title\""))
        assertTrue(create.parametersJson.contains("\"content\""))
        assertTrue(create.parametersJson.contains("\"workspace_id\""))
        assertTrue(create.parametersJson.contains("\"folder\""))
    }

    @Test
    fun unboundLocalToolsIncludeFsAndStage() {
        val defs =
            ToolDispatcher(FsTools(), UnboundForDispatch(), MemoryStorage()).definitions()
        val names = defs.map { it.name }
        assertTrue(names.containsAll(listOf("read", "grep", "write", "edit")))
        assertTrue(names.containsAll(listOf("stage", "list_staged", "get_staged")))
        assertTrue(names.contains("web_search"))
        assertTrue(defs.none { it.name == "create_doc" })
        assertTrue(defs.none { it.name == "delete_staged" })
    }

    @Test
    fun webSearchStaysLocalWhenBound() {
        val storage = MemoryStorage()
        storage.putSecret(SECRET_NAME, "tvly-test")
        val network =
            ScriptedNetwork(
                HttpResponse(
                    status = 200,
                    body =
                        """{"results":[{"title":"Kotlin","url":"https://kotlin.test","content":"lang"}]}"""
                            .encodeToByteArray(),
                ),
            )
        val dispatcher =
            ToolDispatcher(
                FsTools(),
                BoundWmcp(McpTool("web_search", "remote search", "{}")),
                storage,
                WebSearchTools(storage, network),
            )
        val out = dispatcher.call("web_search", """{"query":"kotlin"}""", "/tmp")
        assertTrue(out.contains("https://kotlin.test"))
        assertTrue(out.contains("Kotlin"))
        assertTrue(!out.contains("remote search"))
    }

    @Test
    fun localWebSearchWinsOverRemoteCatalog() {
        val defs =
            ToolDispatcher(
                FsTools(),
                BoundWmcp(McpTool("web_search", "remote search", "{}")),
                MemoryStorage(),
            ).definitions()
        val webs = defs.filter { it.name == "web_search" }
        assertEquals(1, webs.size)
        assertTrue(webs.single().description.contains("public web"))
        assertTrue(!webs.single().description.contains("remote search"))
    }

    @Test
    fun noteContentDescriptionSaysStagedNotBody() {
        val defs =
            ToolDispatcher(
                FsTools(),
                BoundWmcp(
                    McpTool(
                        name = "get_doc_content",
                        description = "Read one document's raw Markdown by id.",
                        inputSchemaJson = """{"type":"object","properties":{"id":{"type":"string"}},"required":["id"]}""",
                    ),
                ),
                MemoryStorage(),
            ).definitions()
        val tool = defs.single { it.name == "get_doc_content" }
        assertEquals(DOC_CONTENT_STAGE_DESCRIPTION, tool.description)
        assertTrue(tool.parametersJson.contains("\"id\""))
    }

    @Test
    fun noteContentSuccessStagesAndHidesBody() {
        val storage = MemoryStorage()
        val docId = "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee"
        val host =
            """{"id":"$docId","ok":true,"content":"# Park doc\n\nsecret body","truncated":false}"""
        val dispatcher =
            ToolDispatcher(
                FsTools(),
                CallingWmcp("get_doc_content", host),
                storage,
            )
        val out =
            dispatcher.call(
                "get_doc_content",
                """{"id":"$docId"}""",
                "/tmp",
                "sess_a",
                "commute chat",
            )
        assertTrue(out.startsWith("success handle=F1 "))
        assertTrue(out.contains("doc_id=$docId"))
        assertTrue(!out.contains("secret body"))
        val body = storage.list("staged").single { it.endsWith("/body.md") }
        assertEquals("# Park doc\n\nsecret body", storage.read(body)?.decodeToString())
        val meta = storage.read(body.removeSuffix("/body.md") + "/meta")?.decodeToString().orEmpty()
        assertTrue(meta.contains("\"source_session_id\":\"sess_a\""))
        assertTrue(meta.contains("\"source_session_title\":\"commute chat\""))
        assertTrue(meta.contains("Park doc"))
    }

    @Test
    fun noteContentFailureDoesNotStage() {
        val storage = MemoryStorage()
        val dispatcher =
            ToolDispatcher(
                FsTools(),
                CallingWmcp(
                    "get_doc_content",
                    """{"id":"ffffffffffffffffffffffffffffffff","ok":false,"error":"Entry not found"}""",
                ),
                storage,
            )
        val out =
            dispatcher.call(
                "get_doc_content",
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
                CallingWmcp("list_notes", """[{"title":"keep"}]"""),
                MemoryStorage(),
            )
        assertEquals(
            """[{"title":"keep"}]""",
            dispatcher.call("list_notes", "{}", "/tmp"),
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

private class ScriptedNetwork(
    private val response: HttpResponse,
) : NetworkClient {
    override fun execute(request: HttpRequest): HttpResponse = response
}

private class UnboundForDispatch : WmcpClient {
    override fun isBound(): Boolean = false

    override fun deviceId(): String = "dev_unbound"

    override fun completeBind(offer: BindOffer, deviceLabel: String?): BindResult = error("unused")

    override fun listTools(): List<McpTool> = emptyList()

    override fun callTool(name: String, arguments: String): McpToolResult = error("unused")
}
