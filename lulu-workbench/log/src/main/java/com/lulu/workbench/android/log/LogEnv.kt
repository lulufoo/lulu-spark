package com.lulu.workbench.android.log

import java.io.File

interface LogEnv {
    val debug: Boolean

    val file: File

    fun nowMs(): Long

    fun pid(): Int

    fun tid(): Int

    fun device(): String

    fun network(): String

    fun writeLogcat(level: LogLevel, moduleId: String, msg: String)
}

interface Logger {
    fun d(msg: String)

    fun i(msg: String)

    fun w(msg: String)

    fun e(msg: String)
}
