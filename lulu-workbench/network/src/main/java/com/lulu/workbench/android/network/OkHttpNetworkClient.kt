package com.lulu.workbench.android.network

import com.lulu.workbench.android.log.LogModule
import com.lulu.workbench.android.log.WbLog
import okhttp3.MediaType.Companion.toMediaTypeOrNull
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.RequestBody.Companion.toRequestBody
import java.security.SecureRandom
import java.security.cert.X509Certificate
import javax.net.ssl.SSLContext
import javax.net.ssl.X509TrustManager

class OkHttpNetworkClient(
    private val openClient: OkHttpClient = OkHttpClient(),
) : NetworkClient {
    override fun execute(request: HttpRequest): HttpResponse {
        val client = clientFor(request.tlsFingerprint)
        val builder = Request.Builder().url(request.url)
        request.headers.forEach { (name, value) -> builder.header(name, value) }
        val body = request.body?.toRequestBody(contentType(request))
        builder.method(request.method, body)
        return try {
            client.newCall(builder.build()).execute().use { response ->
                HttpResponse(
                    status = response.code,
                    body = response.body?.bytes() ?: ByteArray(0),
                    headers = response.headers.names().associate { name ->
                        name.lowercase() to (response.header(name) ?: "")
                    },
                )
            }
        } catch (error: Exception) {
            log.e("execute ${request.method} failed")
            throw error
        }
    }

    private fun contentType(request: HttpRequest) =
        request.headers.entries
            .firstOrNull { it.key.equals("Content-Type", ignoreCase = true) }
            ?.value
            ?.toMediaTypeOrNull()

    private fun clientFor(fingerprint: String?): OkHttpClient {
        if (fingerprint.isNullOrBlank()) return openClient
        val trust = PinTrustManager(fingerprint)
        val ssl = SSLContext.getInstance("TLS")
        ssl.init(null, arrayOf(trust), SecureRandom())
        return openClient.newBuilder()
            .sslSocketFactory(ssl.socketFactory, trust)
            .hostnameVerifier { _, _ -> true }
            .build()
    }
}

private val log = WbLog.module(LogModule.NETWORK)

private class PinTrustManager(
    private val expectedHex: String,
) : X509TrustManager {
    override fun checkClientTrusted(chain: Array<out X509Certificate>, authType: String) = Unit

    override fun checkServerTrusted(chain: Array<out X509Certificate>, authType: String) {
        val leaf = chain.firstOrNull() ?: throw javax.net.ssl.SSLException("empty cert chain")
        val actual = certDerSha256Hex(leaf)
        if (!fingerprintsMatch(expectedHex, actual)) {
            throw javax.net.ssl.SSLException("tls fingerprint mismatch")
        }
    }

    override fun getAcceptedIssuers(): Array<X509Certificate> = emptyArray()
}
