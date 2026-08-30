package com.lulu.workbench.android.asr.tencent

import com.lulu.workbench.android.asr.AsrApiException
import com.lulu.workbench.android.asr.AsrAudioFormat
import com.lulu.workbench.android.asr.AsrClient
import com.lulu.workbench.android.asr.AsrConfig
import com.lulu.workbench.android.asr.AsrException
import com.lulu.workbench.android.asr.AsrHttpException
import com.lulu.workbench.android.asr.AsrNotConfiguredException
import com.lulu.workbench.android.asr.AsrResult
import com.lulu.workbench.android.asr.DefaultAsrEngine
import com.lulu.workbench.android.asr.DefaultAsrRegion
import com.lulu.workbench.android.log.LogModule
import com.lulu.workbench.android.log.WbLog
import com.lulu.workbench.android.network.HttpRequest
import com.lulu.workbench.android.network.NetworkClient
import com.lulu.workbench.android.storage.Storage
import java.io.IOException
import java.net.SocketTimeoutException

internal const val AsrHost = "asr.tencentcloudapi.com"
internal const val AsrUrl = "https://$AsrHost"
internal const val AsrAction = "SentenceRecognition"
internal const val AsrVersion = "2019-06-14"
internal const val AsrService = "asr"
internal const val AsrProfilePath = "asr/tencent"
internal const val AsrSecretName = "asr_tencent_key"
internal const val MaxAsrRawBytes = 2_200_000

internal class AsrClientImpl(
    private val storage: Storage,
    private val network: NetworkClient,
    private val nowSeconds: () -> Long = { System.currentTimeMillis() / 1000L },
) : AsrClient {
    override fun loadConfig(): AsrConfig {
        val raw = storage.read(AsrProfilePath)?.decodeToString().orEmpty()
        return AsrConfig(
            appId = jsonField(raw, "app_id"),
            secretId = jsonField(raw, "secret_id"),
            hasSecretKey = !secretKey().isNullOrBlank(),
            engine = jsonField(raw, "engine").ifBlank { DefaultAsrEngine },
        )
    }

    override fun saveConfig(
        appId: String,
        secretId: String,
        secretKey: String,
        engine: String,
    ) {
        val kept = loadConfig()
        val nextEngine = engine.trim().ifBlank { DefaultAsrEngine }
        storage.write(
            AsrProfilePath,
            encodeProfile(appId.trim(), secretId.trim(), nextEngine).encodeToByteArray(),
        )
        if (secretKey.isNotBlank()) {
            storage.putSecret(AsrSecretName, secretKey.trim())
        } else if (!kept.hasSecretKey) {
            storage.deleteSecret(AsrSecretName)
        }
    }

    override fun clearConfig() {
        storage.delete(AsrProfilePath)
        storage.deleteSecret(AsrSecretName)
    }

    override fun isConfigured(): Boolean {
        val config = loadConfig()
        return config.secretId.isNotBlank() && !secretKey().isNullOrBlank()
    }

    override fun recognize(audio: ByteArray, format: AsrAudioFormat): AsrResult {
        val config = loadConfig()
        val key = secretKey().orEmpty()
        if (config.secretId.isBlank() || key.isBlank()) {
            log.w("recognize skipped: not configured")
            throw AsrNotConfiguredException()
        }
        if (audio.isEmpty()) throw AsrException("asr audio is empty")
        if (audio.size > MaxAsrRawBytes) throw AsrException("asr audio too large")
        val payload = encodeSentenceBody(config.engine, format.wire, audio)
        val signed = signTc3(
            Tc3SignRequest(
                secretId = config.secretId,
                secretKey = key,
                service = AsrService,
                host = AsrHost,
                payload = payload,
                timestampSeconds = nowSeconds(),
            ),
        )
        val response = try {
            network.execute(
                HttpRequest(
                    method = "POST",
                    url = AsrUrl,
                    headers = mapOf(
                        "Content-Type" to "application/json",
                        "Host" to AsrHost,
                        "X-TC-Action" to AsrAction,
                        "X-TC-Version" to AsrVersion,
                        "X-TC-Timestamp" to signed.timestamp,
                        "X-TC-Region" to DefaultAsrRegion,
                        "Authorization" to signed.authorization,
                    ),
                    body = payload.encodeToByteArray(),
                ),
            )
        } catch (_: SocketTimeoutException) {
            log.w("recognize timeout")
            throw AsrException("asr timeout")
        } catch (error: IOException) {
            log.w("recognize network ${error.javaClass.simpleName}")
            throw AsrException("asr network failed")
        }
        val raw = response.body.decodeToString()
        if (response.status !in 200..299) {
            log.w("recognize http ${response.status}")
            throw AsrHttpException(response.status)
        }
        val code = jsonField(raw, "Code")
        val message = jsonField(raw, "Message")
        if (code.isNotBlank()) {
            log.w("recognize api $code")
            throw AsrApiException(code, message)
        }
        val text = jsonField(raw, "Result")
        val requestId = jsonField(raw, "RequestId")
        log.i("recognize ok bytes=${audio.size} chars=${text.length}")
        return AsrResult(text = text, requestId = requestId)
    }

    private fun secretKey(): String? = storage.getSecret(AsrSecretName)?.trim()?.ifBlank { null }
}

internal fun encodeSentenceBody(engine: String, format: String, audio: ByteArray): String {
    val data = encodeBase64(audio)
    return """{"EngSerViceType":"${escapeJson(engine)}","SourceType":1,""" +
        """"VoiceFormat":"${escapeJson(format)}","Data":"$data","DataLen":${audio.size}}"""
}

internal fun encodeProfile(appId: String, secretId: String, engine: String): String =
    """{"app_id":"${escapeJson(appId)}","secret_id":"${escapeJson(secretId)}",""" +
        """"engine":"${escapeJson(engine)}"}"""

internal fun jsonField(json: String, key: String): String {
    val needle = "\"$key\""
    val at = json.indexOf(needle)
    if (at < 0) return ""
    val colon = json.indexOf(':', startIndex = at + needle.length)
    if (colon < 0) return ""
    val quote = json.indexOf('"', startIndex = colon + 1)
    if (quote < 0) return ""
    return unescapeJson(readJsonString(json, quote))
}

private fun readJsonString(source: String, openQuote: Int): String {
    val out = StringBuilder()
    var i = openQuote + 1
    while (i < source.length) {
        val c = source[i]
        if (c == '\\' && i + 1 < source.length) {
            out.append(source[i + 1])
            i += 2
            continue
        }
        if (c == '"') break
        out.append(c)
        i += 1
    }
    return out.toString()
}

private fun escapeJson(value: String): String =
    value.replace("\\", "\\\\").replace("\"", "\\\"").replace("\n", "\\n")

private fun unescapeJson(value: String): String =
    value.replace("\\n", "\n").replace("\\\"", "\"").replace("\\\\", "\\")

internal fun encodeBase64(bytes: ByteArray): String {
    val alphabet = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/"
    val out = StringBuilder((bytes.size + 2) / 3 * 4)
    var i = 0
    while (i < bytes.size) {
        val b0 = bytes[i].toInt() and 0xFF
        val has1 = i + 1 < bytes.size
        val has2 = i + 2 < bytes.size
        val b1 = if (has1) bytes[i + 1].toInt() and 0xFF else 0
        val b2 = if (has2) bytes[i + 2].toInt() and 0xFF else 0
        out.append(alphabet[b0 shr 2])
        out.append(alphabet[((b0 and 3) shl 4) or (b1 shr 4)])
        out.append(if (has1) alphabet[((b1 and 15) shl 2) or (b2 shr 6)] else '=')
        out.append(if (has2) alphabet[b2 and 63] else '=')
        i += 3
    }
    return out.toString()
}

private val log = WbLog.module(LogModule.ASR)
