package com.lulu.workbench.android.bind.state

import com.lulu.workbench.android.bind.commands.BindCommands
import com.lulu.workbench.android.wmcp.BindFailedException
import com.lulu.workbench.android.wmcp.BindOffer
import com.lulu.workbench.android.wmcp.BindResult
import com.lulu.workbench.android.wmcp.McpTool
import com.lulu.workbench.android.wmcp.McpToolResult
import com.lulu.workbench.android.wmcp.WmcpClient
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class BindStoreTest {
    @Test
    fun scannedQrCompletesBind() {
        val wmcp = FakeWmcp()
        val store = BindStore(BindCommands(wmcp, "Pixel"))
        store.dispatch(BindIntent.StartScan)
        assertTrue(store.state.scanning)
        store.dispatch(
            BindIntent.Scanned(
                """{"ip":"10.0.0.2","port":7654,"temp_pub":"aa","tls_fingerprint":"ff","exp":9,"sig":"ss"}""",
            ),
        )
        assertTrue(store.state.bound)
        assertFalse(store.state.scanning)
        assertEquals("dev_testphone1", store.state.deviceId)
        assertEquals(1, wmcp.completes)
    }

    @Test
    fun queryExposesDeviceId() {
        val store = BindStore(BindCommands(FakeWmcp(), "Pixel"))
        store.dispatch(BindIntent.Query)
        assertEquals("dev_testphone1", store.state.deviceId)
        assertFalse(store.state.bound)
    }

    @Test
    fun scannedQrSurfacesFailure() {
        val store = BindStore(BindCommands(FakeWmcp(fail = true), "Pixel"))
        store.dispatch(BindIntent.Scanned("{}"))
        assertFalse(store.state.bound)
        assertTrue(store.state.error.isNotEmpty())
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
