package com.lulu.workbench.android.wmcp.keepalive

import com.lulu.workbench.android.wmcp.mcp.McpWire
import com.lulu.workbench.android.wmcp.mcp.McpWireEvent
import com.lulu.workbench.android.wmcp.mcp.McpWireListener
import java.util.concurrent.CopyOnWriteArrayList

internal class FakeMcpWire(
    var bound: Boolean = false,
    var onActivate: () -> Unit = {},
    var onProbe: () -> Unit = {},
) : McpWire {
    private val listeners = CopyOnWriteArrayList<McpWireListener>()

    override fun isBound(): Boolean = bound

    override fun activate() {
        onActivate()
    }

    override fun probe() {
        onProbe()
    }

    override fun addWireListener(listener: McpWireListener) {
        listeners.add(listener)
    }

    override fun removeWireListener(listener: McpWireListener) {
        listeners.remove(listener)
    }

    fun emit(event: McpWireEvent) {
        listeners.forEach { it.onMcpWire(event) }
    }
}
