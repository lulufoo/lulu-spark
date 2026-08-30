package com.lulu.workbench.android.wmcp.bind

import com.lulu.workbench.android.wmcp.shared.jsonString
import org.bouncycastle.crypto.agreement.X25519Agreement
import org.bouncycastle.crypto.digests.SHA256Digest
import org.bouncycastle.crypto.generators.HKDFBytesGenerator
import org.bouncycastle.crypto.modes.ChaCha20Poly1305
import org.bouncycastle.crypto.params.AEADParameters
import org.bouncycastle.crypto.params.HKDFParameters
import org.bouncycastle.crypto.params.KeyParameter
import org.bouncycastle.crypto.params.X25519PrivateKeyParameters
import org.bouncycastle.crypto.params.X25519PublicKeyParameters
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
    val hostPub = X25519PublicKeyParameters(hexDecode32(tempPubHex))
    val eph = X25519PrivateKeyParameters(SecureRandom())
    val ephPub = eph.generatePublicKey().encoded
    val key = deriveSealKey(sharedSecret(eph, hostPub))
    val nonce = ByteArray(NONCE_LEN).also { SecureRandom().nextBytes(it) }
    val plaintext = bindPlaintextJson(deviceId, deviceLabel).encodeToByteArray()
    val ct = aead(encrypt = true, key = key, nonce = nonce, input = plaintext)
    return ephPub + nonce + ct
}

internal fun openBindRequest(hostSecret: ByteArray, encrypted: ByteArray): BindPlaintext {
    require(encrypted.size >= 32 + NONCE_LEN + 16) { "bind ciphertext too short" }
    val ephPub = X25519PublicKeyParameters(encrypted.copyOfRange(0, 32))
    val nonce = encrypted.copyOfRange(32, 32 + NONCE_LEN)
    val ct = encrypted.copyOfRange(32 + NONCE_LEN, encrypted.size)
    val host = X25519PrivateKeyParameters(hostSecret)
    val key = deriveSealKey(sharedSecret(host, ephPub))
    val plain = aead(encrypt = false, key = key, nonce = nonce, input = ct).decodeToString()
    return BindPlaintext(
        v = jsonString(plain, "v"),
        deviceId = jsonString(plain, "device_id"),
        deviceLabel = runCatching { jsonString(plain, "device_label") }.getOrNull(),
    )
}

internal fun bindOfferExpired(exp: Long, nowSecs: Long): Boolean = nowSecs > exp

private fun sharedSecret(
    privateKey: X25519PrivateKeyParameters,
    publicKey: X25519PublicKeyParameters,
): ByteArray {
    val agreement = X25519Agreement()
    agreement.init(privateKey)
    val shared = ByteArray(agreement.agreementSize)
    agreement.calculateAgreement(publicKey, shared, 0)
    return shared
}

private fun deriveSealKey(shared: ByteArray): ByteArray {
    val hkdf = HKDFBytesGenerator(SHA256Digest())
    hkdf.init(HKDFParameters(shared, null, SEAL_INFO))
    val key = ByteArray(32)
    hkdf.generateBytes(key, 0, 32)
    return key
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

private fun hexDecode32(text: String): ByteArray {
    require(text.length == 64) { "temp_pub must be 32 bytes hex" }
    return text.chunked(2).map { it.toInt(16).toByte() }.toByteArray()
}

private fun escapeJson(value: String): String =
    value.replace("\\", "\\\\").replace("\"", "\\\"")

private const val NONCE_LEN = 12
private val SEAL_INFO = "lulu-workbench-bind-v1".encodeToByteArray()
