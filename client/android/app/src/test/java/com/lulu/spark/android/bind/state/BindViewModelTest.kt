package com.lulu.spark.android.bind.state

import com.lulu.spark.android.bind.BindViewModel
import com.lulu.spark.android.bind.commands.BindCommands
import com.lulu.spark.android.wmcp.BindFailedException
import com.lulu.spark.android.wmcp.BindOffer
import com.lulu.spark.android.wmcp.BindResult
import com.lulu.spark.android.wmcp.McpKeepAlive
import com.lulu.spark.android.wmcp.McpLinkListener
import com.lulu.spark.android.wmcp.McpLinkState
import com.lulu.spark.android.wmcp.McpTool
import com.lulu.spark.android.wmcp.McpToolResult
import com.lulu.spark.android.wmcp.WmcpClient
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class BindViewModelTest {
    @Test
    fun scannedQrCompletesBind() {
        val wmcp = FakeWmcp()
        val store = BindViewModel(BindCommands(wmcp, "Pixel"))
        store.dispatch(BindIntent.StartScan)
        assertTrue(store.state.value.scanning)
        store.dispatch(
            BindIntent.Scanned(
                """{"ip":"10.0.0.2","port":7654,"temp_pub":"aa","tls_fingerprint":"ff","exp":9,"sig":"ss"}""",
            ),
        )
        assertTrue(store.state.value.bound)
        assertFalse(store.state.value.scanning)
        assertEquals("dev_testphone1", store.state.value.deviceId)
        assertEquals(1, wmcp.completes)
    }

    @Test
    fun queryExposesDeviceId() {
        val store = BindViewModel(BindCommands(FakeWmcp(), "Pixel"))
        store.dispatch(BindIntent.Query)
        assertEquals("dev_testphone1", store.state.value.deviceId)
        assertFalse(store.state.value.bound)
    }

    @Test
    fun scannedQrSurfacesFailure() {
        val store = BindViewModel(BindCommands(FakeWmcp(fail = true), "Pixel"))
        store.dispatch(BindIntent.Scanned("{}"))
        assertFalse(store.state.value.bound)
        assertTrue(store.state.value.error.isNotEmpty())
    }

    @Test
    fun keepAliveReplaySetsLinkWithoutStart() {
        val keep = RecordingKeepAlive(McpLinkState.Connected)
        val store = BindViewModel(BindCommands(FakeWmcp(), "Pixel"), keepAlive = keep)
        assertEquals(McpLinkState.Connected, store.state.value.mcpLink)
        assertEquals(0, keep.starts)
        keep.emit(McpLinkState.Disconnected)
        assertEquals(McpLinkState.Disconnected, store.state.value.mcpLink)
        assertEquals(0, keep.starts)
        store.release()
    }
}

private class FakeWmcp(
    private val fail: Boolean = false,
) : WmcpClient {
    var completes = 0

    override fun isBound(): Boolean = completes > 0

    override fun deviceId(): String = "dev_testphone1"

    override fun completeBind(offer: BindOffer, deviceLabel: String?): BindResult {
        if (fail) throw BindFailedException("bind failed")
        completes += 1
        return BindResult("tok")
    }

    override fun listTools(): List<McpTool> = emptyList()

    override fun callTool(name: String, arguments: String): McpToolResult = error("unused")
}

private class RecordingKeepAlive(
    private var current: McpLinkState,
) : McpKeepAlive {
    var starts = 0
    private val listeners = mutableListOf<McpLinkListener>()

    override fun addListener(listener: McpLinkListener) {
        listeners.add(listener)
        listener.onMcpLink(current)
    }

    override fun removeListener(listener: McpLinkListener) {
        listeners.remove(listener)
    }

    override fun start() {
        starts += 1
    }

    override fun state(): McpLinkState = current

    fun emit(next: McpLinkState) {
        current = next
        listeners.forEach { it.onMcpLink(next) }
    }
}
