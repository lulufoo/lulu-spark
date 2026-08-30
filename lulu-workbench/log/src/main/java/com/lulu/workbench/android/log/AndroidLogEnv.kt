package com.lulu.workbench.android.log

import com.lulu.workbench.android.log.format.logFileName

import android.content.Context
import android.net.ConnectivityManager
import android.net.NetworkCapabilities
import android.os.Build
import android.os.Process
import android.util.Log
import java.io.File

class AndroidLogEnv(
    context: Context,
    filesDir: File,
    override val debug: Boolean,
) : LogEnv {
    private val app = context.applicationContext
    private val logDir: File = File(filesDir, "logs")

    override fun fileFor(tsMs: Long): File = File(logDir, logFileName(tsMs))

    override fun nowMs(): Long = System.currentTimeMillis()

    override fun pid(): Int = Process.myPid()

    override fun tid(): Int = Process.myTid()

    override fun device(): String =
        "${Build.MANUFACTURER}/${Build.MODEL}/${Build.VERSION.SDK_INT}"

    override fun network(): String {
        val cm = app.getSystemService(Context.CONNECTIVITY_SERVICE) as? ConnectivityManager
            ?: return "unknown"
        val active = cm.activeNetwork ?: return "none"
        val caps = cm.getNetworkCapabilities(active) ?: return "unknown"
        return when {
            caps.hasTransport(NetworkCapabilities.TRANSPORT_WIFI) -> "wifi"
            caps.hasTransport(NetworkCapabilities.TRANSPORT_CELLULAR) -> "cellular"
            caps.hasTransport(NetworkCapabilities.TRANSPORT_ETHERNET) -> "ethernet"
            else -> "other"
        }
    }

    override fun writeLogcat(level: LogLevel, moduleId: String, msg: String) {
        val tag = "WB/$moduleId"
        when (level) {
            LogLevel.D -> Log.d(tag, msg)
            LogLevel.I -> Log.i(tag, msg)
            LogLevel.W -> Log.w(tag, msg)
            LogLevel.E -> Log.e(tag, msg)
        }
    }
}

