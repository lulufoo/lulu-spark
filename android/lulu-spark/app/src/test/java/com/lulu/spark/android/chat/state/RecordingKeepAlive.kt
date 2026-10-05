package com.lulu.spark.android.chat.state

import com.lulu.spark.android.wmcp.McpKeepAlive
import com.lulu.spark.android.wmcp.McpLinkListener
import com.lulu.spark.android.wmcp.McpLinkState

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
