package com.lulu.spark.android.wmcp.bind

import com.lulu.spark.android.wmcp.BindOffer
import com.lulu.spark.android.wmcp.formatBindCanonical
import org.bouncycastle.crypto.params.Ed25519PrivateKeyParameters
import org.bouncycastle.crypto.params.X25519PrivateKeyParameters
import org.bouncycastle.crypto.signers.Ed25519Signer
import java.security.SecureRandom

internal fun ByteArray.toHex(): String = joinToString("") { byte -> "%02x".format(byte) }

internal fun randomTempPubHex(): String =
    X25519PrivateKeyParameters(SecureRandom()).generatePublicKey().encoded.toHex()

internal fun signedBindOffer(
    ip: String = "10.0.0.2",
    port: Int = 7654,
    tlsFingerprint: String = "ff",
    exp: Long = 4_102_444_800,
): BindOffer {
    val seed = Ed25519PrivateKeyParameters(SecureRandom())
    val unsigned = BindOffer(
        ip = ip,
        port = port,
        tempPub = randomTempPubHex(),
        tlsFingerprint = tlsFingerprint,
        exp = exp,
        signPub = seed.generatePublicKey().encoded.toHex(),
        sig = "",
    )
    val message = formatBindCanonical(unsigned).encodeToByteArray()
    val signer = Ed25519Signer()
    signer.init(true, seed)
    signer.update(message, 0, message.size)
    return unsigned.copy(sig = signer.generateSignature().toHex())
}
