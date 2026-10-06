package com.lulu.spark.android.wmcp.bind

import com.lulu.spark.android.wmcp.BindOffer
import com.lulu.spark.android.wmcp.formatBindCanonical
import org.bouncycastle.crypto.AsymmetricCipherKeyPair
import org.bouncycastle.crypto.digests.SHA256Digest
import org.bouncycastle.crypto.engines.RSAEngine
import org.bouncycastle.crypto.generators.RSAKeyPairGenerator
import org.bouncycastle.crypto.params.RSAKeyGenerationParameters
import org.bouncycastle.crypto.signers.PSSSigner
import org.bouncycastle.crypto.util.PrivateKeyInfoFactory
import org.bouncycastle.crypto.util.SubjectPublicKeyInfoFactory
import java.math.BigInteger
import java.security.SecureRandom

internal fun ByteArray.toHex(): String = joinToString("") { byte -> "%02x".format(byte) }

internal fun generateBindRsa(): AsymmetricCipherKeyPair {
    val gen = RSAKeyPairGenerator()
    gen.init(RSAKeyGenerationParameters(BigInteger.valueOf(65537), SecureRandom(), 2048, 80))
    return gen.generateKeyPair()
}

internal fun tempPubHex(pair: AsymmetricCipherKeyPair): String =
    SubjectPublicKeyInfoFactory.createSubjectPublicKeyInfo(pair.public).encoded.toHex()

internal fun privatePkcs8(pair: AsymmetricCipherKeyPair): ByteArray =
    PrivateKeyInfoFactory.createPrivateKeyInfo(pair.private).encoded

internal fun signedBindOffer(
    ip: String = "10.0.0.2",
    port: Int = 7654,
    tlsFingerprint: String = "ff",
    exp: Long = 4_102_444_800,
): BindOffer {
    val pair = generateBindRsa()
    val unsigned = BindOffer(
        ip = ip,
        port = port,
        tempPub = tempPubHex(pair),
        tlsFingerprint = tlsFingerprint,
        exp = exp,
        sig = "",
    )
    val message = formatBindCanonical(unsigned).encodeToByteArray()
    val signer = PSSSigner(RSAEngine(), SHA256Digest(), 32)
    signer.init(true, pair.private)
    signer.update(message, 0, message.size)
    return unsigned.copy(sig = signer.generateSignature().toHex())
}
