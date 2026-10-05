package com.lulu.spark.android.wmcp.mcp

import com.lulu.spark.android.log.LogModule
import com.lulu.spark.android.log.WbLog
import com.lulu.spark.android.network.HttpRequest
import com.lulu.spark.android.network.NetworkClient
import com.lulu.spark.android.storage.Storage
import com.lulu.spark.android.wmcp.BindFailedException
import com.lulu.spark.android.wmcp.BindOffer
import com.lulu.spark.android.wmcp.BindResult
import com.lulu.spark.android.wmcp.McpFailedException
import com.lulu.spark.android.wmcp.McpTool
import com.lulu.spark.android.wmcp.McpToolResult
import com.lulu.spark.android.wmcp.WmcpClient
import com.lulu.spark.android.wmcp.bind.bindOfferExpired
import com.lulu.spark.android.wmcp.bind.sealBindRequest
import com.lulu.spark.android.wmcp.shared.jsonString
import java.util.UUID
import java.util.concurrent.CopyOnWriteArrayList
import java.util.concurrent.atomic.AtomicInteger

internal class WmcpClientImpl(
    private val storage: Storage,
    private val network: NetworkClient,
) : WmcpClient, McpWire {
    @Volatile
    private var sessionId: String? = null
    private val sessionLock = Any()
    private val nextId = AtomicInteger(1)
    private val wireListeners = CopyOnWriteArrayList<McpWireListener>()

    override fun isBound(): Boolean = !storage.getSecret(DEVICE_TOKEN_SECRET).isNullOrEmpty()

    override fun activate() {
        ensureSession()
    }

    override fun probe() {
        if (!isBound()) throw McpFailedException("not bound")
        ensureSession()
        mcpPost("tools/list", "{}")
    }

    override fun addWireListener(listener: McpWireListener) {
        wireListeners.add(listener)
    }

    override fun removeWireListener(listener: McpWireListener) {
        wireListeners.remove(listener)
    }

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
        synchronized(sessionLock) { sessionId = null }
        emit(McpWireEvent.BindChanged)
        log.i("bind complete")
        return BindResult(deviceMcpToken = token)
    }

    override fun listTools(): List<McpTool> {
        if (!isBound()) return emptyList()
        return try {
            ensureSession()
            parseTools(mcpPost("tools/list", "{}"))
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

    override fun deviceId(): String {
        val existing = storage.getSecret(DEVICE_ID_SECRET)
        if (!existing.isNullOrEmpty()) return existing
        val id = "dev_" + UUID.randomUUID().toString().replace("-", "").take(16)
        storage.putSecret(DEVICE_ID_SECRET, id)
        return id
    }

    private fun ensureSession() {
        synchronized(sessionLock) {
            if (sessionId != null) return
            try {
                val opened = mcpExchange(mcpInitializeBody(nextId.getAndIncrement()), withSession = false)
                initializeAccepted(opened.body)
                sessionId = headerValue(opened.headers, "mcp-session-id")?.trim()?.ifEmpty { null }
                mcpExchange(mcpInitializedBody(), withSession = sessionId != null)
                emit(McpWireEvent.LinkUp)
                log.i("mcp session ready")
            } catch (error: Exception) {
                markLinkDown()
                throw error
            }
        }
    }

    private fun mcpPost(method: String, params: String): String {
        val exchange = mcpExchange(
            mcpRequestBody(nextId.getAndIncrement(), method, params),
            withSession = sessionId != null,
        )
        emit(McpWireEvent.LinkUp)
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
        val response = try {
            network.execute(
                HttpRequest(
                    method = "POST",
                    url = "https://$host:$port/mcp/mobile",
                    headers = headers,
                    body = payload.encodeToByteArray(),
                    tlsFingerprint = fingerprint,
                ),
            )
        } catch (error: Exception) {
            markLinkDown()
            throw error
        }
        if (response.status == 404 && withSession && sessionId != null && retryOnStale) {
            sessionId = null
            ensureSession()
            return mcpExchange(payload, withSession = sessionId != null, retryOnStale = false)
        }
        if (response.status !in 200..299) {
            log.e("mcp post failed status=${response.status}")
            markLinkDown()
            throw McpFailedException("mcp post failed ${response.status}")
        }
        return McpExchange(
            body = response.body.decodeToString(),
            headers = response.headers,
        )
    }

    private fun markLinkDown() {
        sessionId = null
        emit(McpWireEvent.LinkDown)
    }

    private fun emit(event: McpWireEvent) {
        wireListeners.forEach { it.onMcpWire(event) }
    }
}

private data class McpExchange(
    val body: String,
    val headers: Map<String, String>,
)

private val log = WbLog.module(LogModule.WMCP)

private const val DEVICE_TOKEN_SECRET = "device_mcp_token"
private const val DEVICE_ID_SECRET = "device_id"
private const val HOST_PATH = "wmcp/host"
private const val PORT_PATH = "wmcp/port"
private const val FINGERPRINT_PATH = "wmcp/tls_fingerprint"
