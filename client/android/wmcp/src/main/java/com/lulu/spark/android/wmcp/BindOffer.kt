package com.lulu.spark.android.wmcp

import com.lulu.spark.android.wmcp.shared.jsonString
import org.bouncycastle.crypto.digests.SHA256Digest
import org.bouncycastle.crypto.engines.RSAEngine
import org.bouncycastle.crypto.params.RSAKeyParameters
import org.bouncycastle.crypto.signers.PSSSigner
import org.bouncycastle.crypto.util.PublicKeyFactory

data class BindOffer(
    val ip: String,
    val port: Int,
    val tempPub: String,
    val tlsFingerprint: String,
    val exp: Long,
    val sig: String,
)

fun parseBindOffer(qrJson: String): BindOffer {
    return BindOffer(
        ip = jsonString(qrJson, "ip"),
        port = jsonString(qrJson, "port").toInt(),
        tempPub = jsonString(qrJson, "temp_pub"),
        tlsFingerprint = jsonString(qrJson, "tls_fingerprint"),
        exp = jsonString(qrJson, "exp").toLong(),
        sig = jsonString(qrJson, "sig"),
    )
}

fun formatBindCanonical(offer: BindOffer): String =
    "v1|${offer.ip}|${offer.port}|${offer.tempPub}|${offer.tlsFingerprint}|${offer.exp}"

fun verifyBindOffer(offer: BindOffer) {
    val pub = rsaPublic(offer.tempPub)
    val signature = try {
        hexDecodeExact(offer.sig, 256, "sig")
    } catch (_: Exception) {
        throw BindFailedException("bind offer signature rejected")
    }
    val message = formatBindCanonical(offer).encodeToByteArray()
    val verifier = PSSSigner(RSAEngine(), SHA256Digest(), 32)
    verifier.init(false, pub)
    verifier.update(message, 0, message.size)
    if (!verifier.verifySignature(signature)) {
        throw BindFailedException("bind offer signature rejected")
    }
}

internal fun rsaPublic(tempPubHex: String): RSAKeyParameters {
    return try {
        PublicKeyFactory.createKey(hexDecode(tempPubHex)) as RSAKeyParameters
    } catch (_: Exception) {
        throw BindFailedException("bind offer signature rejected")
    }
}

private fun hexDecodeExact(text: String, byteLen: Int, field: String): ByteArray {
    require(text.length == byteLen * 2) { "$field must be $byteLen bytes hex" }
    return hexDecode(text)
}

private fun hexDecode(text: String): ByteArray {
    require(text.length % 2 == 0) { "hex length must be even" }
    return text.chunked(2).map { it.toInt(16).toByte() }.toByteArray()
}
