package com.lulu.workbench.android.log.format

import com.lulu.workbench.android.log.LogLevel

import java.text.SimpleDateFormat
import java.util.Date
import java.util.Locale
import java.util.TimeZone

/**
 * One JSON object per line. Base fields only.
 * Business keys (session, request) stay inside [msg].
 */
internal fun formatLogLine(
    tsMs: Long,
    level: LogLevel,
    moduleId: String,
    pid: Int,
    tid: Int,
    device: String,
    network: String,
    msg: String,
): String {
    val text = if (msg.length > MSG_MAX) msg.take(MSG_MAX) else msg
    return "{" +
        "\"ts\":\"${escape(isoUtc(tsMs))}\"," +
        "\"lvl\":\"${level.name}\"," +
        "\"mod\":\"${escape(moduleId)}\"," +
        "\"pid\":$pid," +
        "\"tid\":$tid," +
        "\"dev\":\"${escape(device)}\"," +
        "\"net\":\"${escape(network)}\"," +
        "\"msg\":\"${escape(text)}\"" +
        "}"
}

internal fun isoUtc(tsMs: Long): String {
    val format = SimpleDateFormat("yyyy-MM-dd'T'HH:mm:ss.SSS'Z'", Locale.US)
    format.timeZone = TimeZone.getTimeZone("UTC")
    return format.format(Date(tsMs))
}

internal fun logFileName(tsMs: Long, timeZone: TimeZone = TimeZone.getDefault()): String {
    val format = SimpleDateFormat("yyyy-MM-dd", Locale.US)
    format.timeZone = timeZone
    return format.format(Date(tsMs)) + "-workbench.jsonl"
}

private fun escape(value: String): String =
    value
        .replace("\\", "\\\\")
        .replace("\"", "\\\"")
        .replace("\n", "\\n")
        .replace("\r", "\\r")

private const val MSG_MAX = 2048
