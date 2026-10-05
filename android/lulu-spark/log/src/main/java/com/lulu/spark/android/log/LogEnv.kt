package com.lulu.spark.android.log

import java.io.File

interface LogEnv {
    val debug: Boolean

    fun fileFor(tsMs: Long): File

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
