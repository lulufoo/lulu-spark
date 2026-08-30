package com.lulu.workbench.android.wmcp.bind

import org.bouncycastle.crypto.params.X25519PrivateKeyParameters
import java.security.SecureRandom

internal fun ByteArray.toHex(): String = joinToString("") { byte -> "%02x".format(byte) }

internal fun randomTempPubHex(): String =
    X25519PrivateKeyParameters(SecureRandom()).generatePublicKey().encoded.toHex()
