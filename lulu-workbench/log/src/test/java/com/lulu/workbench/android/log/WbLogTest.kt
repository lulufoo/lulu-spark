package com.lulu.workbench.android.log

import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test
import java.nio.file.Files

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
        val lines = env.file.readLines()
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
        assertTrue(env.file.readText().contains("quiet"))
        assertEquals(0, env.logcat.size)
    }

    @Test
    fun emitBeforeStartIsIgnored() {
        val env = MemoryEnv(debug = true)
        WbLog.module(LogModule.LLM).e("nope")
        assertEquals(false, env.file.exists())
    }
}

private class MemoryEnv(
    override val debug: Boolean,
) : LogEnv {
    override val file = Files.createTempDirectory("wb-log").resolve("workbench.jsonl").toFile()
    val logcat = mutableListOf<String>()

    override fun nowMs(): Long = 0

    override fun pid(): Int = 1

    override fun tid(): Int = 2

    override fun device(): String = "Test/Dev/1"

    override fun network(): String = "wifi"

    override fun writeLogcat(level: LogLevel, moduleId: String, msg: String) {
        logcat.add("$level $moduleId $msg")
    }
}
