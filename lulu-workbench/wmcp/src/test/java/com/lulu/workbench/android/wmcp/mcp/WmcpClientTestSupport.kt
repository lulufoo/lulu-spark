package com.lulu.workbench.android.wmcp.mcp

import com.lulu.workbench.android.network.HttpRequest
import com.lulu.workbench.android.network.HttpResponse
import com.lulu.workbench.android.network.NetworkClient
import com.lulu.workbench.android.storage.MemoryStorage
import com.lulu.workbench.android.storage.Storage
import com.lulu.workbench.android.wmcp.BindOffer
import com.lulu.workbench.android.wmcp.WmcpClient
import com.lulu.workbench.android.wmcp.bind.randomTempPubHex
import com.lulu.workbench.android.wmcp.keepalive.LinkKeepAliveClock
import com.lulu.workbench.android.wmcp.keepalive.ManualLinkClock
import com.lulu.workbench.android.wmcp.keepalive.assembleWmcpClient

internal const val LIST_TOOLS_WITH_SCHEMA =
    """{"result":{"tools":[{"name":"create_note","description":"Create a note from Markdown content.","inputSchema":{"type":"object","properties":{"title":{"type":"string"},"content":{"type":"string"},"project":{"type":"string"}},"required":["content","title"],"additionalProperties":false}}]}}"""

internal fun liveOffer(): BindOffer =
    BindOffer(
        ip = "10.0.0.2",
        port = 7654,
        tempPub = randomTempPubHex(),
        tlsFingerprint = "ff",
        exp = 4_102_444_800,
        sig = "ss",
    )

internal fun wmcpClient(
    network: RecordingNetwork,
    clock: LinkKeepAliveClock = ManualLinkClock(),
    storage: Storage = MemoryStorage(),
): WmcpClient = assembleWmcpClient(storage, network, clock)

internal class RecordingNetwork(
    vararg responses: HttpResponse,
) : NetworkClient {
    private val queue = responses.toMutableList()
    val requests = mutableListOf<HttpRequest>()

    override fun execute(request: HttpRequest): HttpResponse {
        requests.add(request)
        return queue.removeFirst()
    }
}
