package com.lulu.spark.android.wmcp.keepalive

import com.lulu.spark.android.wmcp.McpLinkState
import com.lulu.spark.android.wmcp.mcp.McpWireEvent
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class McpKeepAliveTest {
    @Test
    fun unboundStartDoesNotActivate() {
        var activates = 0
        val clock = ManualLinkClock()
        val wire = FakeMcpWire(bound = false, onActivate = { activates += 1 })
        val keep = McpKeepAliveImpl(wire, clock)
        val seen = mutableListOf<McpLinkState>()
        keep.addListener { seen.add(it) }
        keep.start()
        assertEquals(0, activates)
        assertEquals(0, clock.executeCount)
        assertEquals(McpLinkState.Unbound, keep.state())
        assertEquals(listOf(McpLinkState.Unbound), seen)
    }

    @Test
    fun boundStartRetriesEveryTenSecondsUntilUp() {
        var activates = 0
        val clock = ManualLinkClock()
        val wire = FakeMcpWire(
            bound = true,
            onActivate = {
                activates += 1
                if (activates < 3) error("down")
            },
        )
        val keep = McpKeepAliveImpl(wire, clock)
        keep.start()
        assertEquals(1, activates)
        assertEquals(McpLinkState.Disconnected, keep.state())
        clock.advance(MCP_KEEP_ALIVE_RETRY_MS)
        assertEquals(2, activates)
        assertEquals(McpLinkState.Disconnected, keep.state())
        clock.advance(MCP_KEEP_ALIVE_RETRY_MS)
        assertEquals(3, activates)
        assertEquals(McpLinkState.Connected, keep.state())
        clock.advance(MCP_KEEP_ALIVE_RETRY_MS)
        assertEquals(3, activates)
        assertEquals(McpLinkState.Connected, keep.state())
    }

    @Test
    fun conversationLinkUpArmsProbe() {
        var probes = 0
        val clock = ManualLinkClock()
        val wire = FakeMcpWire(
            bound = true,
            onActivate = { error("down") },
            onProbe = { probes += 1 },
        )
        val keep = McpKeepAliveImpl(wire, clock)
        keep.start()
        assertEquals(1, clock.pending())
        wire.emit(McpWireEvent.LinkUp)
        assertEquals(McpLinkState.Connected, keep.state())
        clock.advance(MCP_KEEP_ALIVE_RETRY_MS)
        assertEquals(1, probes)
        assertEquals(McpLinkState.Connected, keep.state())
    }

    @Test
    fun connectedProbeFailureMarksDisconnected() {
        var probes = 0
        val clock = ManualLinkClock()
        val wire = FakeMcpWire(
            bound = true,
            onProbe = {
                probes += 1
                error("down")
            },
        )
        val keep = McpKeepAliveImpl(wire, clock)
        keep.start()
        assertEquals(McpLinkState.Connected, keep.state())
        clock.advance(MCP_KEEP_ALIVE_RETRY_MS)
        assertEquals(1, probes)
        assertEquals(McpLinkState.Disconnected, keep.state())
    }

    @Test
    fun conversationLinkDownRestartsRetry() {
        var activates = 0
        val clock = ManualLinkClock()
        val wire = FakeMcpWire(bound = true, onActivate = { activates += 1 })
        val keep = McpKeepAliveImpl(wire, clock)
        keep.start()
        assertEquals(McpLinkState.Connected, keep.state())
        wire.emit(McpWireEvent.LinkDown)
        assertEquals(McpLinkState.Disconnected, keep.state())
        clock.advance(MCP_KEEP_ALIVE_RETRY_MS)
        assertEquals(2, activates)
        assertEquals(McpLinkState.Connected, keep.state())
    }

    @Test
    fun bindAfterStartActivates() {
        var activates = 0
        val clock = ManualLinkClock()
        val wire = FakeMcpWire(bound = false, onActivate = { activates += 1 })
        val keep = McpKeepAliveImpl(wire, clock)
        keep.start()
        assertEquals(0, activates)
        wire.bound = true
        wire.emit(McpWireEvent.BindChanged)
        assertEquals(1, activates)
        assertEquals(McpLinkState.Connected, keep.state())
    }

    @Test
    fun addListenerReplaysCurrentState() {
        val wire = FakeMcpWire(bound = true)
        val keep = McpKeepAliveImpl(wire, ManualLinkClock())
        keep.start()
        val seen = mutableListOf<McpLinkState>()
        keep.addListener { seen.add(it) }
        assertEquals(listOf(McpLinkState.Connected), seen)
    }

    @Test
    fun startIsIdempotent() {
        var activates = 0
        val wire = FakeMcpWire(bound = true, onActivate = { activates += 1 })
        val keep = McpKeepAliveImpl(wire, ManualLinkClock())
        keep.start()
        keep.start()
        assertEquals(1, activates)
        assertTrue(keep.state() == McpLinkState.Connected)
    }
}
