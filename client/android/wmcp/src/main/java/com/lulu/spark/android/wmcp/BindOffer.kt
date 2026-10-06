package com.lulu.spark.android.wmcp

import com.lulu.spark.android.wmcp.shared.jsonString
import org.bouncycastle.crypto.params.Ed25519PublicKeyParameters
import org.bouncycastle.crypto.signers.Ed25519Signer

data class BindOffer(
    val ip: String,
    val port: Int,
    val tempPub: String,
    val tlsFingerprint: String,
    val exp: Long,
    val signPub: String,
    val sig: String,
)

fun parseBindOffer(qrJson: String): BindOffer {
    return BindOffer(
        ip = jsonString(qrJson, "ip"),
        port = jsonString(qrJson, "port").toInt(),
        tempPub = jsonString(qrJson, "temp_pub"),
        tlsFingerprint = jsonString(qrJson, "tls_fingerprint"),
        exp = jsonString(qrJson, "exp").toLong(),
        signPub = jsonString(qrJson, "sign_pub"),
        sig = jsonString(qrJson, "sig"),
    )
}

fun formatBindCanonical(offer: BindOffer): String =
    "v1|${offer.ip}|${offer.port}|${offer.tempPub}|${offer.tlsFingerprint}|${offer.exp}"

fun verifyBindOffer(offer: BindOffer) {
    val pub = hexDecodeExact(offer.signPub, 32, "sign_pub")
    val signature = hexDecodeExact(offer.sig, 64, "sig")
    val message = formatBindCanonical(offer).encodeToByteArray()
    val verifier = Ed25519Signer()
    verifier.init(false, Ed25519PublicKeyParameters(pub))
    verifier.update(message, 0, message.size)
    if (!verifier.verifySignature(signature)) {
        throw BindFailedException("bind offer signature rejected")
    }
}

private fun hexDecodeExact(text: String, byteLen: Int, field: String): ByteArray {
    require(text.length == byteLen * 2) { "$field must be $byteLen bytes hex" }
    return text.chunked(2).map { it.toInt(16).toByte() }.toByteArray()
}
