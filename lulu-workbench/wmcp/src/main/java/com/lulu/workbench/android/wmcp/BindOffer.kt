package com.lulu.workbench.android.wmcp

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

internal fun jsonString(json: String, key: String): String {
    val needle = "\"$key\""
    val at = json.indexOf(needle)
    require(at >= 0) { "missing $key" }
    val colon = json.indexOf(':', startIndex = at + needle.length)
    var i = colon + 1
    while (i < json.length && json[i].isWhitespace()) i += 1
    if (i < json.length && json[i] == '"') {
        val end = json.indexOf('"', startIndex = i + 1)
        return json.substring(i + 1, end)
    }
    val end = json.indexOfFirstFrom(i) { ch -> ch == ',' || ch == '}' || ch.isWhitespace() }
    return json.substring(i, end)
}

private fun String.indexOfFirstFrom(start: Int, predicate: (Char) -> Boolean): Int {
    var i = start
    while (i < length && !predicate(this[i])) i += 1
    return i
}
