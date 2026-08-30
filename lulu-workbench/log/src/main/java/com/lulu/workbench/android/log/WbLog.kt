package com.lulu.workbench.android.log

import com.lulu.workbench.android.log.format.formatLogLine
import java.io.File

internal const val LOG_PATH_CACHE_MS = 60_000L

object WbLog {
    @Volatile
    private var env: LogEnv? = null
    private val fileLock = Any()
    private var cachedFile: File? = null
    private var cachedAtMs: Long = 0L

    fun start(env: LogEnv) {
        this.env = env
        val now = env.nowMs()
        synchronized(fileLock) {
            val file = env.fileFor(now)
            file.parentFile?.mkdirs()
            cachedFile = file
            cachedAtMs = now
        }
        module(LogModule.LOG).i("started debug=${env.debug}")
    }

    fun module(moduleId: String): Logger = ModuleLogger(moduleId)

    internal fun reset() {
        env = null
        synchronized(fileLock) {
            cachedFile = null
            cachedAtMs = 0L
        }
    }

    internal fun emit(level: LogLevel, moduleId: String, msg: String) {
        val current = env ?: return
        val now = current.nowMs()
        val line =
            formatLogLine(
                tsMs = now,
                level = level,
                moduleId = moduleId,
                pid = current.pid(),
                tid = current.tid(),
                device = current.device(),
                network = current.network(),
                msg = msg,
            )
        synchronized(fileLock) {
            val target = resolveFile(current, now)
            target.parentFile?.mkdirs()
            target.appendText(line + "\n")
        }
        if (current.debug) {
            current.writeLogcat(level, moduleId, msg)
        }
    }

    private fun resolveFile(current: LogEnv, nowMs: Long): File {
        val cached = cachedFile
        if (cached != null && nowMs - cachedAtMs < LOG_PATH_CACHE_MS) {
            return cached
        }
        val fresh = current.fileFor(nowMs)
        cachedFile = fresh
        cachedAtMs = nowMs
        return fresh
    }
}

private class ModuleLogger(
    private val moduleId: String,
) : Logger {
    override fun d(msg: String) = WbLog.emit(LogLevel.D, moduleId, msg)

    override fun i(msg: String) = WbLog.emit(LogLevel.I, moduleId, msg)

    override fun w(msg: String) = WbLog.emit(LogLevel.W, moduleId, msg)

    override fun e(msg: String) = WbLog.emit(LogLevel.E, moduleId, msg)
}
