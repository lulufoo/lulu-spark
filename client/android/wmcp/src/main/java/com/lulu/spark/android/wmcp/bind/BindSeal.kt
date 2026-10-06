package com.lulu.spark.android.wmcp.bind

import com.lulu.spark.android.wmcp.rsaPublic
import com.lulu.spark.android.wmcp.shared.jsonString
import org.bouncycastle.crypto.digests.SHA256Digest
import org.bouncycastle.crypto.encodings.OAEPEncoding
import org.bouncycastle.crypto.engines.RSAEngine
import org.bouncycastle.crypto.modes.ChaCha20Poly1305
import org.bouncycastle.crypto.params.AEADParameters
import org.bouncycastle.crypto.params.KeyParameter
import org.bouncycastle.crypto.params.RSAKeyParameters
import org.bouncycastle.crypto.util.PrivateKeyFactory
import java.security.SecureRandom

data class BindPlaintext(
    val v: String,
    val deviceId: String,
    val deviceLabel: String?,
)

fun sealBindRequest(
    tempPubHex: String,
    deviceId: String,
    deviceLabel: String? = null,
): ByteArray {
    val hostPub = rsaPublic(tempPubHex)
    val key = ByteArray(SYM_KEY_LEN).also { SecureRandom().nextBytes(it) }
    val wrapped = rsaOaep(encrypt = true, key = hostPub, input = key)
    val nonce = ByteArray(NONCE_LEN).also { SecureRandom().nextBytes(it) }
    val plaintext = bindPlaintextJson(deviceId, deviceLabel).encodeToByteArray()
    val ct = aead(encrypt = true, key = key, nonce = nonce, input = plaintext)
    return wrapped + nonce + ct
}

internal fun openBindRequest(hostSecret: ByteArray, encrypted: ByteArray): BindPlaintext {
    require(encrypted.size >= OAEP_CT_LEN + NONCE_LEN + 16) { "bind ciphertext too short" }
    val priv = PrivateKeyFactory.createKey(hostSecret) as RSAKeyParameters
    val key = rsaOaep(encrypt = false, key = priv, input = encrypted.copyOfRange(0, OAEP_CT_LEN))
    val nonce = encrypted.copyOfRange(OAEP_CT_LEN, OAEP_CT_LEN + NONCE_LEN)
    val ct = encrypted.copyOfRange(OAEP_CT_LEN + NONCE_LEN, encrypted.size)
    val plain = aead(encrypt = false, key = key, nonce = nonce, input = ct).decodeToString()
    return BindPlaintext(
        v = jsonString(plain, "v"),
        deviceId = jsonString(plain, "device_id"),
        deviceLabel = runCatching { jsonString(plain, "device_label") }.getOrNull(),
    )
}

internal fun bindOfferExpired(exp: Long, nowSecs: Long): Boolean = nowSecs > exp

private fun rsaOaep(encrypt: Boolean, key: RSAKeyParameters, input: ByteArray): ByteArray {
    val engine = OAEPEncoding(RSAEngine(), SHA256Digest())
    engine.init(encrypt, key)
    return engine.processBlock(input, 0, input.size)
}

private fun aead(encrypt: Boolean, key: ByteArray, nonce: ByteArray, input: ByteArray): ByteArray {
    val cipher = ChaCha20Poly1305()
    cipher.init(encrypt, AEADParameters(KeyParameter(key), 128, nonce))
    val out = ByteArray(cipher.getOutputSize(input.size))
    val n = cipher.processBytes(input, 0, input.size, out, 0)
    cipher.doFinal(out, n)
    return out
}

private fun bindPlaintextJson(deviceId: String, deviceLabel: String?): String {
    val id = escapeJson(deviceId)
    if (deviceLabel.isNullOrBlank()) {
        return """{"v":"1","device_id":"$id"}"""
    }
    return """{"v":"1","device_id":"$id","device_label":"${escapeJson(deviceLabel)}"}"""
}

private fun escapeJson(value: String): String =
    value.replace("\\", "\\\\").replace("\"", "\\\"")

private const val NONCE_LEN = 12
private const val OAEP_CT_LEN = 256
private const val SYM_KEY_LEN = 32
