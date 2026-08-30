package com.lulu.workbench.android.network

import com.lulu.workbench.android.network.okhttp.OkHttpNetworkClient

/** Outbound HTTPS only. Does not parse bind or MCP meaning. */
data class HttpRequest(
    val method: String,
    val url: String,
    val headers: Map<String, String> = emptyMap(),
    val body: ByteArray? = null,
    val tlsFingerprint: String? = null,
)

data class HttpResponse(
    val status: Int,
    val body: ByteArray,
    val headers: Map<String, String> = emptyMap(),
)

interface NetworkClient {
    fun execute(request: HttpRequest): HttpResponse

    fun executeStream(request: HttpRequest, onChunk: (ByteArray) -> Unit): HttpResponse {
        throw UnsupportedOperationException("stream is reserved; mvp uses execute")
    }
}

object NetworkFactory {
    fun create(): NetworkClient = OkHttpNetworkClient()
}
