package com.lulu.workbench.android.bind.state

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import com.lulu.workbench.android.bind.commands.BindCommands
import com.lulu.workbench.android.wmcp.IdleKeepAlive
import com.lulu.workbench.android.wmcp.McpKeepAlive
import com.lulu.workbench.android.wmcp.McpLinkListener

class BindStore(
    private val commands: BindCommands,
    private val keepAlive: McpKeepAlive = IdleKeepAlive,
    private val runOffMain: (() -> Unit) -> Unit = { it() },
    private val runOnMain: (() -> Unit) -> Unit = { it() },
) {
    var state: BindState by mutableStateOf(BindState())
        private set

    private val linkListener = McpLinkListener { next ->
        runOnMain {
            if (state.mcpLink != next) {
                state = state.copy(mcpLink = next)
            }
        }
    }

    init {
        keepAlive.addListener(linkListener)
    }

    fun release() {
        keepAlive.removeListener(linkListener)
    }

    fun dispatch(intent: BindIntent) {
        when (intent) {
            BindIntent.Query ->
                state = state.copy(
                    bound = commands.isBound(),
                    deviceId = commands.deviceId(),
                    scanning = false,
                    completing = false,
                )
            BindIntent.StartScan ->
                state = state.copy(scanning = true, error = "")
            BindIntent.CancelScan ->
                state = state.copy(scanning = false)
            BindIntent.CameraDenied ->
                state = state.copy(scanning = false, error = "camera permission denied")
            is BindIntent.Scanned -> {
                if (state.completing) return
                state = state.copy(scanning = false, completing = true, error = "")
                runOffMain {
                    try {
                        commands.completeFromQr(intent.qr)
                        runOnMain {
                            state = state.copy(
                                bound = true,
                                deviceId = commands.deviceId(),
                                scanning = false,
                                completing = false,
                                error = "",
                            )
                        }
                    } catch (error: Exception) {
                        runOnMain {
                            state = state.copy(
                                bound = commands.isBound(),
                                deviceId = commands.deviceId(),
                                scanning = false,
                                completing = false,
                                error = error.message ?: "bind failed",
                            )
                        }
                    }
                }
            }
        }
    }
}
