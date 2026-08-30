package com.lulu.workbench.android.wmcp

import com.lulu.workbench.android.network.HttpRequest
import com.lulu.workbench.android.network.HttpResponse
import com.lulu.workbench.android.network.NetworkClient
import com.lulu.workbench.android.storage.MemoryStorage
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class WmcpClientTest {
    @Test
    fun parseOfferReadsQrFields() {
        val offer = parseBindOffer(
            """{"ip":"10.0.0.2","port":7654,"temp_pub":"aa","tls_fingerprint":"ff","exp":1,"sig":"ss"}""",
        )
        assertEquals("10.0.0.2", offer.ip)
        assertEquals(7654, offer.port)
        assertEquals("ff", offer.tlsFingerprint)
    }

    @Test
    fun deviceIdIsStablePlaintext() {
        val storage = MemoryStorage()
        val wmcp = WmcpClientImpl(storage, RecordingNetwork())
        val first = wmcp.deviceId()
        assertTrue(first.startsWith("dev_"))
        assertEquals(20, first.length)
        assertEquals(first, wmcp.deviceId())
        assertEquals(first, WmcpClientImpl(storage, RecordingNetwork()).deviceId())
    }

    @Test
    fun unboundListToolsDoesNotCallNetwork() {
        val network = RecordingNetwork()
        val wmcp = WmcpClientImpl(MemoryStorage(), network)
        assertTrue(wmcp.listTools().isEmpty())
        assertTrue(network.requests.isEmpty())
    }

    @Test
    fun completeBindSealsAndPostsToGateway() {
        val network = RecordingNetwork(
            HttpResponse(200, """{"device_mcp_token":"tok"}""".encodeToByteArray()),
        )
        val wmcp = WmcpClientImpl(MemoryStorage(), network)
        val offer = liveOffer()
        assertEquals("tok", wmcp.completeBind(offer, "Pixel").deviceMcpToken)
        val sent = network.requests.single()
        assertEquals("https://10.0.0.2:7654/bind/complete", sent.url)
        assertEquals("ff", sent.tlsFingerprint)
        assertTrue((sent.body?.size ?: 0) >= 32 + 12 + 16)
        assertTrue(wmcp.isBound())
    }

    @Test(expected = BindFailedException::class)
    fun completeBindRejectsExpiredOffer() {
        val wmcp = WmcpClientImpl(MemoryStorage(), RecordingNetwork())
        wmcp.completeBind(liveOffer().copy(exp = 1))
    }

    @Test
    fun listToolsAfterBindCompletesHandshake() {
        val network = RecordingNetwork(
            HttpResponse(200, """{"device_mcp_token":"tok"}""".encodeToByteArray()),
            HttpResponse(
                200,
                """{"result":{"protocolVersion":"2025-03-26"}}""".encodeToByteArray(),
                mapOf("mcp-session-id" to "sess-1"),
            ),
            HttpResponse(202, ByteArray(0)),
            HttpResponse(200, LIST_TOOLS_WITH_SCHEMA.encodeToByteArray()),
        )
        val wmcp = WmcpClientImpl(MemoryStorage(), network)
        wmcp.completeBind(liveOffer())
        val listed = wmcp.listTools()
        assertEquals(listOf("create_note"), listed.map { it.name })
        assertTrue(listed.single().inputSchemaJson.contains("\"title\""))
        val init = network.requests[1]
        val initialized = network.requests[2]
        val list = network.requests[3]
        assertTrue(init.body!!.decodeToString().contains("\"initialize\""))
        assertTrue(init.headers["Mcp-Session-Id"].isNullOrEmpty())
        assertEquals(MCP_ACCEPT, init.headers["Accept"])
        assertTrue(initialized.body!!.decodeToString().contains("notifications/initialized"))
        assertEquals("sess-1", initialized.headers["Mcp-Session-Id"])
        assertTrue(list.body!!.decodeToString().contains("tools/list"))
        assertEquals("sess-1", list.headers["Mcp-Session-Id"])
        assertEquals("https://10.0.0.2:7654/mcp/mobile", list.url)
    }

    @Test
    fun jsonRpcPayloadReadsSseData() {
        val raw = "event: message\ndata: {\"result\":{\"tools\":[{\"name\":\"get_notes\"}]}}\n\n"
        assertEquals(
            listOf("get_notes"),
            parseToolNames(jsonRpcPayload(raw)),
        )
    }

    @Test
    fun parseToolsKeepsHostInputSchemaAndDescription() {
        val tools = parseTools(LIST_TOOLS_WITH_SCHEMA)
        assertEquals(1, tools.size)
        assertEquals("create_note", tools[0].name)
        assertEquals("Create a note from Markdown content.", tools[0].description)
        assertTrue(tools[0].inputSchemaJson.contains("\"properties\""))
        assertTrue(tools[0].inputSchemaJson.contains("\"title\""))
        assertTrue(tools[0].inputSchemaJson.contains("\"content\""))
        assertTrue(tools[0].inputSchemaJson.contains("\"required\""))
    }

    @Test
    fun parseToolsDoesNotTreatNestedNameAsTool() {
        val body =
            """{"result":{"tools":[{"name":"create_todo_task","description":"Add a task.","inputSchema":{"type":"object","properties":{"title":{"type":"string","description":"name of the task"}}}}]}}"""
        assertEquals(listOf("create_todo_task"), parseToolNames(body))
    }

    @Test
    fun parseToolsFallsBackWhenSchemaMissing() {
        val tools = parseTools("""{"result":{"tools":[{"name":"get_notes"}]}}""")
        assertEquals("get_notes", tools.single().name)
        assertEquals("""{"type":"object"}""", tools.single().inputSchemaJson)
    }

    @Test
    fun listToolsWorksWhenInitializeOmitsSession() {
        val network = RecordingNetwork(
            HttpResponse(200, """{"device_mcp_token":"tok"}""".encodeToByteArray()),
            HttpResponse(200, """{"result":{"protocolVersion":"2025-03-26"}}""".encodeToByteArray()),
            HttpResponse(202, ByteArray(0)),
            HttpResponse(200, """{"result":{"tools":[{"name":"get_notes"}]}}""".encodeToByteArray()),
        )
        val wmcp = WmcpClientImpl(MemoryStorage(), network)
        wmcp.completeBind(liveOffer())
        assertEquals(listOf("get_notes"), wmcp.listTools().map { it.name })
        assertTrue(network.requests[2].headers["Mcp-Session-Id"].isNullOrEmpty())
        assertTrue(network.requests[3].headers["Mcp-Session-Id"].isNullOrEmpty())
    }

    @Test
    fun listToolsReinitializesAfterSession404() {
        val network = RecordingNetwork(
            HttpResponse(200, """{"device_mcp_token":"tok"}""".encodeToByteArray()),
            HttpResponse(
                200,
                """{"result":{"protocolVersion":"2025-03-26"}}""".encodeToByteArray(),
                mapOf("mcp-session-id" to "old"),
            ),
            HttpResponse(202, ByteArray(0)),
            HttpResponse(404, ByteArray(0)),
            HttpResponse(
                200,
                """{"result":{"protocolVersion":"2025-03-26"}}""".encodeToByteArray(),
                mapOf("mcp-session-id" to "new"),
            ),
            HttpResponse(202, ByteArray(0)),
            HttpResponse(200, """{"result":{"tools":[{"name":"get_notes"}]}}""".encodeToByteArray()),
        )
        val wmcp = WmcpClientImpl(MemoryStorage(), network)
        wmcp.completeBind(liveOffer())
        assertEquals(listOf("get_notes"), wmcp.listTools().map { it.name })
        assertEquals("new", network.requests.last().headers["Mcp-Session-Id"])
        assertTrue(network.requests.count { it.body?.decodeToString()?.contains("\"initialize\"") == true } == 2)
    }

    @Test
    fun listToolsRejectsInitializeError() {
        val network = RecordingNetwork(
            HttpResponse(200, """{"device_mcp_token":"tok"}""".encodeToByteArray()),
            HttpResponse(
                200,
                """{"jsonrpc":"2.0","id":1,"error":{"code":-32602,"message":"bad"}}""".encodeToByteArray(),
            ),
        )
        val wmcp = WmcpClientImpl(MemoryStorage(), network)
        wmcp.completeBind(liveOffer())
        assertTrue(wmcp.listTools().isEmpty())
    }

    @Test
    fun listToolsHttpErrorReturnsEmpty() {
        val network = RecordingNetwork(
            HttpResponse(200, """{"device_mcp_token":"tok"}""".encodeToByteArray()),
            HttpResponse(406, ByteArray(0)),
        )
        val wmcp = WmcpClientImpl(MemoryStorage(), network)
        wmcp.completeBind(liveOffer())
        assertTrue(wmcp.listTools().isEmpty())
    }
}

private const val LIST_TOOLS_WITH_SCHEMA =
    """{"result":{"tools":[{"name":"create_note","description":"Create a note from Markdown content.","inputSchema":{"type":"object","properties":{"title":{"type":"string"},"content":{"type":"string"},"project":{"type":"string"}},"required":["content","title"],"additionalProperties":false}}]}}"""

private fun liveOffer(): BindOffer =
    BindOffer(
        ip = "10.0.0.2",
        port = 7654,
        tempPub = randomTempPubHex(),
        tlsFingerprint = "ff",
        exp = 4_102_444_800,
        sig = "ss",
    )

private class RecordingNetwork(
    vararg responses: HttpResponse,
) : NetworkClient {
    private val queue = responses.toMutableList()
    val requests = mutableListOf<HttpRequest>()

    override fun execute(request: HttpRequest): HttpResponse {
        requests.add(request)
        return queue.removeFirst()
    }
}
