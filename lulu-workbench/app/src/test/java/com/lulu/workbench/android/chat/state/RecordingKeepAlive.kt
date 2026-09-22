package com.lulu.workbench.android.chat.state

import com.lulu.workbench.android.wmcp.McpKeepAlive
import com.lulu.workbench.android.wmcp.McpLinkListener
import com.lulu.workbench.android.wmcp.McpLinkState

internal class RecordingKeepAlive(
    private var current: McpLinkState,
) : McpKeepAlive {
    private val listeners = mutableListOf<McpLinkListener>()

    override fun addListener(listener: McpLinkListener) {
        listeners.add(listener)
        listener.onMcpLink(current)
    }

    override fun removeListener(listener: McpLinkListener) {
        listeners.remove(listener)
    }

    override fun start() = Unit

    override fun state(): McpLinkState = current

    fun emit(next: McpLinkState) {
        current = next
        listeners.forEach { it.onMcpLink(next) }
    }
}
