package com.lulu.workbench.android.bind.commands

import com.lulu.workbench.android.wmcp.WmcpClient
import com.lulu.workbench.android.wmcp.parseBindOffer

class BindCommands(
    private val wmcp: WmcpClient,
    private val deviceLabel: String = "Android",
) {
    fun isBound(): Boolean = wmcp.isBound()

    fun completeFromQr(qrJson: String) {
        wmcp.completeBind(parseBindOffer(qrJson), deviceLabel)
    }
}
