package com.lulu.spark.android.log.format

import com.lulu.spark.android.log.LogLevel
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test
import java.util.TimeZone

class LogFormatTest {
    @Test
    fun lineHasBaseFieldsAndNoBusinessKeys() {
        val line =
            formatLogLine(
                tsMs = 0L,
                level = LogLevel.I,
                moduleId = "agent",
                pid = 11,
                tid = 22,
                device = "Google/Pixel/34",
                network = "wifi",
                msg = "send session=sess_1 request=req_1",
            )
        assertTrue(line.startsWith("{"))
        assertTrue(line.contains("\"ts\":\"1970-01-01T00:00:00.000Z\""))
        assertTrue(line.contains("\"lvl\":\"I\""))
        assertTrue(line.contains("\"mod\":\"agent\""))
        assertTrue(line.contains("\"pid\":11"))
        assertTrue(line.contains("\"tid\":22"))
        assertTrue(line.contains("\"dev\":\"Google/Pixel/34\""))
        assertTrue(line.contains("\"net\":\"wifi\""))
        assertTrue(line.contains("\"msg\":\"send session=sess_1 request=req_1\""))
        assertFalse(line.contains("\"session\""))
        assertFalse(line.contains("\"request\""))
        assertTrue(isoUtc(0L).contains("T"))
    }

    @Test
    fun dailyFileNameUsesDatePrefix() {
        val utc = TimeZone.getTimeZone("UTC")
        assertEquals("1970-01-01-spark.jsonl", logFileName(0L, utc))
        assertEquals("2026-08-29-spark.jsonl", logFileName(1_787_961_600_000L, utc))
    }
}
