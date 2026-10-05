package com.lulu.spark.android

import android.app.Application
import android.content.pm.ApplicationInfo
import com.lulu.spark.android.agent.facade.SparkRuntime
import com.lulu.spark.android.log.AndroidLogEnv
import com.lulu.spark.android.log.LogModule
import com.lulu.spark.android.log.WbLog
import dagger.hilt.android.HiltAndroidApp

@HiltAndroidApp
class SparkApp : Application() {
    lateinit var runtime: SparkRuntime
        private set

    override fun onCreate() {
        super.onCreate()
        val debug = applicationInfo.flags and ApplicationInfo.FLAG_DEBUGGABLE != 0
        WbLog.start(AndroidLogEnv(this, filesDir, debug))
        runtime = SparkRuntime.create(filesDir)
        WbLog.module(LogModule.APP).i("app create")
    }
}
