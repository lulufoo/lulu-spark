package com.lulu.spark.android.wmcp

import com.lulu.spark.android.wmcp.shared.jsonString

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
