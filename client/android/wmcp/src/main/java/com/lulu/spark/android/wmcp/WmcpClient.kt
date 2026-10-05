package com.lulu.spark.android.wmcp

import com.lulu.spark.android.network.NetworkFactory
import com.lulu.spark.android.storage.Storage
import com.lulu.spark.android.wmcp.keepalive.assembleWmcpClient

data class BindResult(
    val deviceMcpToken: String,
)

data class McpTool(
    val name: String,
    val description: String,
    val inputSchemaJson: String = """{"type":"object"}""",
)

data class McpToolResult(
    val text: String,
)

class BindFailedException(message: String) : Exception(message)

class McpFailedException(message: String) : Exception(message)

interface WmcpClient {
    fun isBound(): Boolean

    fun deviceId(): String

    fun completeBind(offer: BindOffer, deviceLabel: String? = null): BindResult

    fun listTools(): List<McpTool>

    fun callTool(name: String, arguments: String): McpToolResult

    fun keepAlive(): McpKeepAlive = IdleKeepAlive
}

object WmcpFactory {
    fun create(storage: Storage): WmcpClient =
        assembleWmcpClient(storage, NetworkFactory.create())
}
