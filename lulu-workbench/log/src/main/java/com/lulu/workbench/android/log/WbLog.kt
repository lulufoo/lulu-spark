package com.lulu.workbench.android.log

object WbLog {
    @Volatile
    private var env: LogEnv? = null
    private val fileLock = Any()

    fun start(env: LogEnv) {
        env.file.parentFile?.mkdirs()
        this.env = env
        module(LogModule.LOG).i("started debug=${env.debug}")
    }

    fun module(moduleId: String): Logger = ModuleLogger(moduleId)

    internal fun reset() {
        env = null
    }

    internal fun emit(level: LogLevel, moduleId: String, msg: String) {
        val current = env ?: return
        val line =
            formatLogLine(
                tsMs = current.nowMs(),
                level = level,
                moduleId = moduleId,
                pid = current.pid(),
                tid = current.tid(),
                device = current.device(),
                network = current.network(),
                msg = msg,
            )
        synchronized(fileLock) {
            current.file.appendText(line + "\n")
        }
        if (current.debug) {
            current.writeLogcat(level, moduleId, msg)
        }
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
