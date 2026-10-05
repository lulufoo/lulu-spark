package com.lulu.spark.android.wmcp.mcp

internal enum class McpWireEvent {
    BindChanged,
    LinkUp,
    LinkDown,
}

internal fun interface McpWireListener {
    fun onMcpWire(event: McpWireEvent)
}

/** Session and RPC port. Keep-alive calls these; MCP only reports wire events. */
internal interface McpWire {
    fun isBound(): Boolean

    fun activate()

    fun probe()

    fun addWireListener(listener: McpWireListener)

    fun removeWireListener(listener: McpWireListener)
}
