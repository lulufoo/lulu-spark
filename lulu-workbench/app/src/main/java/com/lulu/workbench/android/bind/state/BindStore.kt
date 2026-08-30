package com.lulu.workbench.android.bind.state

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import com.lulu.workbench.android.bind.commands.BindCommands

class BindStore(
    private val commands: BindCommands,
    private val runOffMain: (() -> Unit) -> Unit = { it() },
    private val runOnMain: (() -> Unit) -> Unit = { it() },
) {
    var state: BindState by mutableStateOf(BindState())
        private set

    fun dispatch(intent: BindIntent) {
        when (intent) {
            BindIntent.Query ->
                state = BindState(
                    bound = commands.isBound(),
                    deviceId = commands.deviceId(),
                    error = state.error,
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
                            state = BindState(
                                bound = true,
                                deviceId = commands.deviceId(),
                            )
                        }
                    } catch (error: Exception) {
                        runOnMain {
                            state = BindState(
                                bound = commands.isBound(),
                                deviceId = commands.deviceId(),
                                error = error.message ?: "bind failed",
                            )
                        }
                    }
                }
            }
        }
    }
}
