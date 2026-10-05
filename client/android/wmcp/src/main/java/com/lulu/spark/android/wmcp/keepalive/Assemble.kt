package com.lulu.spark.android.wmcp.keepalive

import com.lulu.spark.android.network.NetworkClient
import com.lulu.spark.android.storage.Storage
import com.lulu.spark.android.wmcp.McpKeepAlive
import com.lulu.spark.android.wmcp.WmcpClient
import com.lulu.spark.android.wmcp.mcp.WmcpClientImpl

internal fun assembleWmcpClient(
    storage: Storage,
    network: NetworkClient,
    clock: LinkKeepAliveClock = ExecutorLinkClock(),
): WmcpClient {
    val mcp = WmcpClientImpl(storage, network)
    return AssembledWmcpClient(mcp, McpKeepAliveImpl(mcp, clock))
}

private class AssembledWmcpClient(
    private val mcp: WmcpClient,
    private val hub: McpKeepAlive,
) : WmcpClient by mcp {
    override fun keepAlive(): McpKeepAlive = hub
}
