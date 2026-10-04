package com.lulu.spark.android.log

import com.lulu.spark.android.log.format.logFileName
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test
import java.io.File
import java.nio.file.Files
import java.util.TimeZone

class WbLogTest {
    @Before
    fun reset() {
        WbLog.reset()
    }

    @Test
    fun debugWritesFileAndLogcat() {
        val env = MemoryEnv(debug = true)
        WbLog.start(env)
        WbLog.module(LogModule.AGENT).i("hello")
        val lines = env.file().readLines()
        assertTrue(lines.any { it.contains("\"mod\":\"log\"") })
        assertTrue(lines.any { it.contains("\"msg\":\"hello\"") && it.contains("\"mod\":\"agent\"") })
        assertTrue(env.logcat.any { it.contains("hello") })
    }

    @Test
    fun releaseWritesFileOnly() {
        val env = MemoryEnv(debug = false)
        WbLog.start(env)
        env.logcat.clear()
        WbLog.module(LogModule.APP).w("quiet")
        assertTrue(env.file().readText().contains("quiet"))
        assertEquals(0, env.logcat.size)
    }

    @Test
    fun emitBeforeStartIsIgnored() {
        val env = MemoryEnv(debug = true)
        WbLog.module(LogModule.LLM).e("nope")
        assertEquals(false, env.file().exists())
    }

    @Test
    fun writesSeparateFilePerLocalDay() {
        val env = MemoryEnv(debug = false)
        env.nowMsValue = 0L
        WbLog.start(env)
        WbLog.module(LogModule.APP).i("day-one")
        env.nowMsValue = DAY_2026_08_29_UTC
        WbLog.module(LogModule.APP).i("day-two")
        assertTrue(env.file(0L).readText().contains("day-one"))
        assertTrue(env.file(DAY_2026_08_29_UTC).readText().contains("day-two"))
        assertTrue(env.file(0L).name.startsWith("1970-01-01"))
        assertTrue(env.file(DAY_2026_08_29_UTC).name.startsWith("2026-08-29"))
    }

    @Test
    fun reusesCachedPathUntilTimeout() {
        val env = MemoryEnv(debug = false)
        WbLog.start(env)
        WbLog.module(LogModule.APP).i("a")
        env.nowMsValue = LOG_PATH_CACHE_MS - 1
        WbLog.module(LogModule.APP).i("b")
        assertEquals(1, env.fileForCalls)
    }

    @Test
    fun refreshesPathAfterTimeout() {
        val env = MemoryEnv(debug = false)
        WbLog.start(env)
        assertEquals(1, env.fileForCalls)
        env.nowMsValue = LOG_PATH_CACHE_MS
        WbLog.module(LogModule.APP).i("after")
        assertEquals(2, env.fileForCalls)
        assertTrue(env.file().readText().contains("after"))
    }
}

private class MemoryEnv(
    override val debug: Boolean,
) : LogEnv {
    private val dir = Files.createTempDirectory("wb-log").toFile()
    val logcat = mutableListOf<String>()
    var nowMsValue: Long = 0
    var fileForCalls: Int = 0

    fun file(tsMs: Long = nowMsValue) = File(dir, logFileName(tsMs, UTC))

    override fun fileFor(tsMs: Long): File {
        fileForCalls += 1
        return file(tsMs)
    }

    override fun nowMs(): Long = nowMsValue

    override fun pid(): Int = 1

    override fun tid(): Int = 2

    override fun device(): String = "Test/Dev/1"

    override fun network(): String = "wifi"

    override fun writeLogcat(level: LogLevel, moduleId: String, msg: String) {
        logcat.add("$level $moduleId $msg")
    }
}

private val UTC: TimeZone = TimeZone.getTimeZone("UTC")

private const val DAY_2026_08_29_UTC = 1_787_961_600_000L
