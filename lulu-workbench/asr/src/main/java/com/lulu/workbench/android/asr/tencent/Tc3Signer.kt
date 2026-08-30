package com.lulu.workbench.android.asr.tencent

import java.security.MessageDigest
import java.text.SimpleDateFormat
import java.util.Date
import java.util.Locale
import java.util.TimeZone
import javax.crypto.Mac
import javax.crypto.spec.SecretKeySpec

internal data class Tc3SignRequest(
    val secretId: String,
    val secretKey: String,
    val service: String,
    val host: String,
    val payload: String,
    val timestampSeconds: Long,
)

internal data class Tc3SignResult(
    val authorization: String,
    val timestamp: String,
    val date: String,
)

internal fun signTc3(request: Tc3SignRequest): Tc3SignResult {
    val date = utcDate(request.timestampSeconds)
    val hashedPayload = sha256Hex(request.payload.toByteArray(Charsets.UTF_8))
    val canonicalHeaders = "content-type:application/json\nhost:${request.host}\n"
    val signedHeaders = "content-type;host"
    val canonicalRequest =
        "POST\n/\n\n$canonicalHeaders\n$signedHeaders\n$hashedPayload"
    val credentialScope = "$date/${request.service}/tc3_request"
    val stringToSign =
        "TC3-HMAC-SHA256\n${request.timestampSeconds}\n$credentialScope\n" +
            sha256Hex(canonicalRequest.toByteArray(Charsets.UTF_8))
    val secretDate = hmacSha256(("TC3" + request.secretKey).toByteArray(Charsets.UTF_8), date)
    val secretService = hmacSha256(secretDate, request.service)
    val secretSigning = hmacSha256(secretService, "tc3_request")
    val signature = hmacSha256(secretSigning, stringToSign).toHex()
    val authorization =
        "TC3-HMAC-SHA256 Credential=${request.secretId}/$credentialScope, " +
            "SignedHeaders=$signedHeaders, Signature=$signature"
    return Tc3SignResult(
        authorization = authorization,
        timestamp = request.timestampSeconds.toString(),
        date = date,
    )
}

private fun utcDate(timestampSeconds: Long): String {
    val format = SimpleDateFormat("yyyy-MM-dd", Locale.US)
    format.timeZone = TimeZone.getTimeZone("UTC")
    return format.format(Date(timestampSeconds * 1000L))
}

private fun sha256Hex(bytes: ByteArray): String =
    MessageDigest.getInstance("SHA-256").digest(bytes).toHex()

private fun hmacSha256(key: ByteArray, message: String): ByteArray {
    val mac = Mac.getInstance("HmacSHA256")
    mac.init(SecretKeySpec(key, "HmacSHA256"))
    return mac.doFinal(message.toByteArray(Charsets.UTF_8))
}

private fun ByteArray.toHex(): String = joinToString("") { byte ->
    "%02x".format(byte)
}
