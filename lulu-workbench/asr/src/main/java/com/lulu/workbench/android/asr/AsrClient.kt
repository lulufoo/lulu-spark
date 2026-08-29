package com.lulu.workbench.android.asr

const val DefaultAsrEngine = "16k_zh"
const val DefaultAsrRegion = "ap-guangzhou"

enum class AsrAudioFormat(val wire: String) {
    Wav("wav"),
    Pcm("pcm"),
    Mp3("mp3"),
}

data class AsrConfig(
    val appId: String = "",
    val secretId: String = "",
    val hasSecretKey: Boolean = false,
    val engine: String = DefaultAsrEngine,
)

data class AsrResult(
    val text: String,
    val requestId: String = "",
)

open class AsrException(message: String) : Exception(message)

class AsrNotConfiguredException : AsrException("asr is not configured")

class AsrHttpException(val status: Int) : AsrException("asr http $status")

class AsrApiException(val code: String, detail: String) : AsrException("asr $code $detail")

/**
 * Tencent Cloud one-sentence ASR ([SentenceRecognition](https://cloud.tencent.com/document/product/1093/35646)).
 * Credentials come from settings via [Storage] secrets. Audio must be ≤ 60s / 3MB.
 */
interface AsrClient {
    fun loadConfig(): AsrConfig

    fun saveConfig(appId: String, secretId: String, secretKey: String, engine: String = DefaultAsrEngine)

    fun clearConfig()

    fun isConfigured(): Boolean

    fun recognize(audio: ByteArray, format: AsrAudioFormat = AsrAudioFormat.Wav): AsrResult
}
