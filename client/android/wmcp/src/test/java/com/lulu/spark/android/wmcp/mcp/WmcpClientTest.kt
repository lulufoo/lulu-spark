package com.lulu.spark.android.wmcp.mcp

import com.lulu.spark.android.network.HttpResponse
import com.lulu.spark.android.storage.MemoryStorage
import com.lulu.spark.android.wmcp.BindFailedException
import com.lulu.spark.android.wmcp.McpLinkState
import com.lulu.spark.android.wmcp.keepalive.MCP_KEEP_ALIVE_RETRY_MS
import com.lulu.spark.android.wmcp.keepalive.ManualLinkClock
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class WmcpClientTest {
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
        assertTrue((sent.body?.size ?: 0) >= 256 + 12 + 16)
        assertTrue(wmcp.isBound())
    }

    @Test(expected = BindFailedException::class)
    fun completeBindRejectsExpiredOffer() {
        val wmcp = WmcpClientImpl(MemoryStorage(), RecordingNetwork())
        wmcp.completeBind(liveOffer().copy(exp = 1))
    }

    @Test(expected = BindFailedException::class)
    fun completeBindRejectsBadSignature() {
        val wmcp = WmcpClientImpl(MemoryStorage(), RecordingNetwork())
        wmcp.completeBind(liveOffer().copy(sig = "00".repeat(512)))
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
        val wmcp = wmcpClient(network)
        wmcp.completeBind(liveOffer())
        assertTrue(wmcp.listTools().isEmpty())
        assertEquals(McpLinkState.Disconnected, wmcp.keepAlive().state())
    }

    @Test
    fun listToolsSuccessMarksConnected() {
        val network = RecordingNetwork(
            HttpResponse(200, """{"device_mcp_token":"tok"}""".encodeToByteArray()),
            HttpResponse(
                200,
                """{"result":{"protocolVersion":"2025-03-26"}}""".encodeToByteArray(),
                mapOf("mcp-session-id" to "sess-1"),
            ),
            HttpResponse(202, ByteArray(0)),
            HttpResponse(200, """{"result":{"tools":[{"name":"get_notes"}]}}""".encodeToByteArray()),
        )
        val wmcp = wmcpClient(network)
        wmcp.completeBind(liveOffer())
        assertEquals(McpLinkState.Unbound, wmcp.keepAlive().state())
        wmcp.listTools()
        assertEquals(McpLinkState.Connected, wmcp.keepAlive().state())
    }

    @Test
    fun callToolJsonRpcErrorDoesNotDisconnect() {
        val network = RecordingNetwork(
            HttpResponse(200, """{"device_mcp_token":"tok"}""".encodeToByteArray()),
            HttpResponse(
                200,
                """{"result":{"protocolVersion":"2025-03-26"}}""".encodeToByteArray(),
                mapOf("mcp-session-id" to "sess-1"),
            ),
            HttpResponse(202, ByteArray(0)),
            HttpResponse(
                200,
                """{"jsonrpc":"2.0","id":2,"error":{"code":-32602,"message":"bad args"}}""".encodeToByteArray(),
            ),
        )
        val wmcp = wmcpClient(network)
        wmcp.completeBind(liveOffer())
        val result = wmcp.callTool("create_note", "{}")
        assertTrue(result.text.contains("bad args") || result.text.contains("error"))
        assertEquals(McpLinkState.Connected, wmcp.keepAlive().state())
    }

    @Test
    fun conversationListToolsCancelsKeepAliveRetry() {
        val clock = ManualLinkClock()
        val network = RecordingNetwork(
            HttpResponse(200, """{"device_mcp_token":"tok"}""".encodeToByteArray()),
            HttpResponse(406, ByteArray(0)),
            HttpResponse(
                200,
                """{"result":{"protocolVersion":"2025-03-26"}}""".encodeToByteArray(),
                mapOf("mcp-session-id" to "sess-1"),
            ),
            HttpResponse(202, ByteArray(0)),
            HttpResponse(200, """{"result":{"tools":[{"name":"get_notes"}]}}""".encodeToByteArray()),
            HttpResponse(200, """{"result":{"tools":[{"name":"get_notes"}]}}""".encodeToByteArray()),
        )
        val wmcp = wmcpClient(network, clock)
        wmcp.completeBind(liveOffer())
        wmcp.keepAlive().start()
        assertEquals(McpLinkState.Disconnected, wmcp.keepAlive().state())
        assertEquals(1, clock.pending())
        assertEquals(listOf("get_notes"), wmcp.listTools().map { it.name })
        assertEquals(McpLinkState.Connected, wmcp.keepAlive().state())
        clock.advance(MCP_KEEP_ALIVE_RETRY_MS)
        assertEquals(McpLinkState.Connected, wmcp.keepAlive().state())
        assertEquals(2, clock.executeCount)
    }

    @Test
    fun listToolsLinkFailureDoesNotFlipGreenOnStaleSessionRetry() {
        val clock = ManualLinkClock()
        val network = RecordingNetwork(
            HttpResponse(200, """{"device_mcp_token":"tok"}""".encodeToByteArray()),
            HttpResponse(
                200,
                """{"result":{"protocolVersion":"2025-03-26"}}""".encodeToByteArray(),
                mapOf("mcp-session-id" to "sess-1"),
            ),
            HttpResponse(202, ByteArray(0)),
            HttpResponse(200, """{"result":{"tools":[{"name":"get_notes"}]}}""".encodeToByteArray()),
            HttpResponse(406, ByteArray(0)),
            HttpResponse(406, ByteArray(0)),
        )
        val wmcp = wmcpClient(network, clock)
        wmcp.completeBind(liveOffer())
        wmcp.keepAlive().start()
        assertEquals(listOf("get_notes"), wmcp.listTools().map { it.name })
        assertEquals(McpLinkState.Connected, wmcp.keepAlive().state())
        assertTrue(wmcp.listTools().isEmpty())
        assertEquals(McpLinkState.Disconnected, wmcp.keepAlive().state())
        clock.advance(MCP_KEEP_ALIVE_RETRY_MS)
        assertEquals(McpLinkState.Disconnected, wmcp.keepAlive().state())
    }

    @Test
    fun keepAliveProbeMarksDisconnectedWhenHostGone() {
        val clock = ManualLinkClock()
        val network = RecordingNetwork(
            HttpResponse(200, """{"device_mcp_token":"tok"}""".encodeToByteArray()),
            HttpResponse(
                200,
                """{"result":{"protocolVersion":"2025-03-26"}}""".encodeToByteArray(),
                mapOf("mcp-session-id" to "sess-1"),
            ),
            HttpResponse(202, ByteArray(0)),
            HttpResponse(200, """{"result":{"tools":[{"name":"get_notes"}]}}""".encodeToByteArray()),
            HttpResponse(406, ByteArray(0)),
        )
        val wmcp = wmcpClient(network, clock)
        wmcp.completeBind(liveOffer())
        wmcp.keepAlive().start()
        assertEquals(listOf("get_notes"), wmcp.listTools().map { it.name })
        assertEquals(McpLinkState.Connected, wmcp.keepAlive().state())
        clock.advance(MCP_KEEP_ALIVE_RETRY_MS)
        assertEquals(McpLinkState.Disconnected, wmcp.keepAlive().state())
    }
}
