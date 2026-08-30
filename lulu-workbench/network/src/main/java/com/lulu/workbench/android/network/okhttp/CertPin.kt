package com.lulu.workbench.android.network.okhttp

import java.security.MessageDigest
import java.security.cert.X509Certificate

internal fun certDerSha256Hex(cert: X509Certificate): String {
    val digest = MessageDigest.getInstance("SHA-256").digest(cert.encoded)
    return digest.joinToString("") { byte -> "%02x".format(byte) }
}

internal fun fingerprintsMatch(expected: String, actualHex: String): Boolean =
    expected.lowercase() == actualHex.lowercase()
