package com.lulu.workbench.android.wmcp

import com.lulu.workbench.android.network.HttpRequest
import com.lulu.workbench.android.network.NetworkClient
import com.lulu.workbench.android.network.NetworkFactory
import com.lulu.workbench.android.log.LogModule
import com.lulu.workbench.android.log.WbLog
import com.lulu.workbench.android.storage.Storage
import java.util.UUID
import java.util.concurrent.atomic.AtomicInteger

data class BindResult(
    val deviceMcpToken: String,
)

data class McpTool(
    val name: String,
    val description: String,
)

data class McpToolResult(
    val text: String,
)

class BindFailedException(message: String) : Exception(message)

class McpFailedException(message: String) : Exception(message)

interface WmcpClient {
    fun isBound(): Boolean

    fun completeBind(offer: BindOffer, deviceLabel: String? = null): BindResult

    fun listTools(): List<McpTool>

    fun callTool(name: String, arguments: String): McpToolResult
}

class WmcpClientImpl(
    private val storage: Storage,
    private val network: NetworkClient,
) : WmcpClient {
    @Volatile
    private var sessionId: String? = null
    private val nextId = AtomicInteger(1)
    override fun isBound(): Boolean = !storage.getSecret(DEVICE_TOKEN_SECRET).isNullOrEmpty()

    override fun completeBind(offer: BindOffer, deviceLabel: String?): BindResult {
        if (bindOfferExpired(offer.exp, System.currentTimeMillis() / 1000)) {
            log.w("bind expired")
            throw BindFailedException("bind offer expired")
        }
        val sealed = sealBindRequest(offer.tempPub, deviceId(), deviceLabel)
        val response = network.execute(
            HttpRequest(
                method = "POST",
                url = "https://${offer.ip}:${offer.port}/bind/complete",
                headers = mapOf("Content-Type" to "application/octet-stream"),
                body = sealed,
                tlsFingerprint = offer.tlsFingerprint,
            ),
        )
        if (response.status !in 200..299) {
            log.e("bind failed status=${response.status}")
            throw BindFailedException("bind failed ${response.status}")
        }
        val token = jsonString(response.body.decodeToString(), "device_mcp_token")
        storage.putSecret(DEVICE_TOKEN_SECRET, token)
        storage.write(HOST_PATH, offer.ip.encodeToByteArray())
        storage.write(PORT_PATH, offer.port.toString().encodeToByteArray())
        storage.write(FINGERPRINT_PATH, offer.tlsFingerprint.encodeToByteArray())
        sessionId = null
        log.i("bind complete")
        return BindResult(deviceMcpToken = token)
    }

    override fun listTools(): List<McpTool> {
        if (!isBound()) return emptyList()
        return try {
            ensureSession()
            parseToolNames(mcpPost("tools/list", "{}")).map { McpTool(name = it, description = "") }
        } catch (error: Exception) {
            log.e("mcp tools/list unavailable")
            emptyList()
        }
    }

    override fun callTool(name: String, arguments: String): McpToolResult {
        if (!isBound()) return McpToolResult(text = "not bound")
        return try {
            ensureSession()
            val params = """{"name":"$name","arguments":$arguments}"""
            McpToolResult(text = mcpPost("tools/call", params))
        } catch (error: Exception) {
            McpToolResult(text = error.message ?: "mcp call failed")
        }
    }

    private fun ensureSession() {
        if (sessionId != null) return
        val opened = mcpExchange(mcpInitializeBody(nextId.getAndIncrement()), withSession = false)
        initializeAccepted(opened.body)
        sessionId = headerValue(opened.headers, "mcp-session-id")?.trim()?.ifEmpty { null }
        mcpExchange(mcpInitializedBody(), withSession = sessionId != null)
        log.i("mcp session ready")
    }

    private fun deviceId(): String {
        val existing = storage.getSecret(DEVICE_ID_SECRET)
        if (!existing.isNullOrEmpty()) return existing
        val id = "dev_" + UUID.randomUUID().toString().replace("-", "").take(16)
        storage.putSecret(DEVICE_ID_SECRET, id)
        return id
    }

    private fun mcpPost(method: String, params: String): String {
        val exchange = mcpExchange(
            mcpRequestBody(nextId.getAndIncrement(), method, params),
            withSession = sessionId != null,
        )
        return jsonRpcPayload(exchange.body)
    }

    private fun mcpExchange(
        payload: String,
        withSession: Boolean,
        retryOnStale: Boolean = true,
    ): McpExchange {
        val host = storage.read(HOST_PATH)?.decodeToString().orEmpty()
        val port = storage.read(PORT_PATH)?.decodeToString().orEmpty()
        val fingerprint = storage.read(FINGERPRINT_PATH)?.decodeToString().orEmpty()
        val token = storage.getSecret(DEVICE_TOKEN_SECRET).orEmpty()
        val headers = mutableMapOf(
            "Authorization" to "Bearer $token",
            "Content-Type" to "application/json",
            "Accept" to MCP_ACCEPT,
            "MCP-Protocol-Version" to MCP_PROTOCOL_VERSION,
        )
        if (withSession) {
            val session = sessionId.orEmpty()
            if (session.isNotEmpty()) {
                headers["Mcp-Session-Id"] = session
            }
        }
        val response = network.execute(
            HttpRequest(
                method = "POST",
                url = "https://$host:$port/mcp/mobile",
                headers = headers,
                body = payload.encodeToByteArray(),
                tlsFingerprint = fingerprint,
            ),
        )
        if (response.status == 404 && withSession && sessionId != null && retryOnStale) {
            sessionId = null
            ensureSession()
            return mcpExchange(payload, withSession = sessionId != null, retryOnStale = false)
        }
        if (response.status !in 200..299) {
            log.e("mcp post failed status=${response.status}")
            throw McpFailedException("mcp post failed ${response.status}")
        }
        return McpExchange(
            body = response.body.decodeToString(),
            headers = response.headers,
        )
    }
}

private data class McpExchange(
    val body: String,
    val headers: Map<String, String>,
)

object WmcpFactory {
    fun create(storage: Storage): WmcpClient = WmcpClientImpl(storage, NetworkFactory.create())
}

internal fun parseToolNames(body: String): List<String> {
    val names = mutableListOf<String>()
    var cursor = 0
    while (true) {
        val key = body.indexOf("\"name\"", startIndex = cursor)
        if (key < 0) return names
        val colon = body.indexOf(':', startIndex = key)
        val quote = body.indexOf('"', startIndex = colon + 1)
        val end = body.indexOf('"', startIndex = quote + 1)
        if (quote < 0 || end < 0) return names
        names.add(body.substring(quote + 1, end))
        cursor = end + 1
    }
}

private val log = WbLog.module(LogModule.WMCP)

private const val DEVICE_TOKEN_SECRET = "device_mcp_token"
private const val DEVICE_ID_SECRET = "device_id"
private const val HOST_PATH = "wmcp/host"
private const val PORT_PATH = "wmcp/port"
private const val FINGERPRINT_PATH = "wmcp/tls_fingerprint"
