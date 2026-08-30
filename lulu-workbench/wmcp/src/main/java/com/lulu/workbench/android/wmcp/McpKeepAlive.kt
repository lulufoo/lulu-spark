package com.lulu.workbench.android.wmcp

enum class McpLinkState {
    Unbound,
    Disconnected,
    Connected,
}

fun interface McpLinkListener {
    fun onMcpLink(state: McpLinkState)
}

/** App-facing keep-alive. Get it from [WmcpClient.keepAlive]; do not construct an implementation. */
interface McpKeepAlive {
    fun addListener(listener: McpLinkListener)

    fun removeListener(listener: McpLinkListener)

    fun start()

    fun state(): McpLinkState
}

object IdleKeepAlive : McpKeepAlive {
    override fun addListener(listener: McpLinkListener) {
        listener.onMcpLink(McpLinkState.Unbound)
    }

    override fun removeListener(listener: McpLinkListener) = Unit

    override fun start() = Unit

    override fun state(): McpLinkState = McpLinkState.Unbound
}
