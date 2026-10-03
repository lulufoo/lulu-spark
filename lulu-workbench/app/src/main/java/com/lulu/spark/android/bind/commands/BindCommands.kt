package com.lulu.spark.android.bind.commands

import android.os.Build
import com.lulu.spark.android.agent.facade.WorkbenchRuntime
import com.lulu.spark.android.wmcp.WmcpClient
import com.lulu.spark.android.wmcp.parseBindOffer
import javax.inject.Inject

class BindCommands(
    private val wmcp: WmcpClient,
    private val deviceLabel: String = "Android",
) {
    @Inject
    constructor(runtime: WorkbenchRuntime) : this(runtime.wmcp, Build.MODEL)

    fun isBound(): Boolean = wmcp.isBound()

    fun deviceId(): String = wmcp.deviceId()

    fun completeFromQr(qrJson: String) {
        wmcp.completeBind(parseBindOffer(qrJson), deviceLabel)
    }
}
