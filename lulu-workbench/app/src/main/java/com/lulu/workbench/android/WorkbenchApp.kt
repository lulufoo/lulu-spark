package com.lulu.workbench.android

import android.app.Application
import android.content.pm.ApplicationInfo
import com.lulu.workbench.android.agent.facade.WorkbenchRuntime
import com.lulu.workbench.android.log.AndroidLogEnv
import com.lulu.workbench.android.log.LogModule
import com.lulu.workbench.android.log.WbLog
import dagger.hilt.android.HiltAndroidApp

@HiltAndroidApp
class WorkbenchApp : Application() {
    lateinit var runtime: WorkbenchRuntime
        private set

    override fun onCreate() {
        super.onCreate()
        val debug = applicationInfo.flags and ApplicationInfo.FLAG_DEBUGGABLE != 0
        WbLog.start(AndroidLogEnv(this, filesDir, debug))
        runtime = WorkbenchRuntime.create(filesDir)
        WbLog.module(LogModule.APP).i("app create")
    }
}
