package com.lulu.spark.android.bind

import android.os.Handler
import android.os.Looper
import androidx.lifecycle.ViewModel
import com.lulu.spark.android.agent.facade.WorkbenchRuntime
import com.lulu.spark.android.bind.commands.BindCommands
import com.lulu.spark.android.bind.state.BindIntent
import com.lulu.spark.android.bind.state.BindState
import com.lulu.spark.android.wmcp.IdleKeepAlive
import com.lulu.spark.android.wmcp.McpKeepAlive
import com.lulu.spark.android.wmcp.McpLinkListener
import dagger.hilt.android.lifecycle.HiltViewModel
import javax.inject.Inject
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update

@HiltViewModel
class BindViewModel(
    private val commands: BindCommands,
    private val keepAlive: McpKeepAlive = IdleKeepAlive,
    private val runOffMain: (() -> Unit) -> Unit = { it() },
    private val runOnMain: (() -> Unit) -> Unit = { it() },
) : ViewModel() {
    @Inject
    constructor(
        commands: BindCommands,
        runtime: WorkbenchRuntime,
    ) : this(
        commands,
        keepAlive = runtime.wmcp.keepAlive(),
        runOffMain = { block -> Thread { block() }.start() },
        runOnMain = { block -> Handler(Looper.getMainLooper()).post(block) },
    )

    private val _state = MutableStateFlow(BindState())
    val state: StateFlow<BindState> = _state.asStateFlow()

    private val linkListener = McpLinkListener { next ->
        runOnMain {
            if (_state.value.mcpLink != next) {
                _state.update { it.copy(mcpLink = next) }
            }
        }
    }

    init {
        keepAlive.addListener(linkListener)
    }

    fun release() {
        keepAlive.removeListener(linkListener)
    }

    override fun onCleared() {
        release()
        super.onCleared()
    }

    fun dispatch(intent: BindIntent) {
        when (intent) {
            BindIntent.Query ->
                _state.update {
                    it.copy(
                        bound = commands.isBound(),
                        deviceId = commands.deviceId(),
                        scanning = false,
                        completing = false,
                    )
                }
            BindIntent.StartScan ->
                _state.update { it.copy(scanning = true, error = "") }
            BindIntent.CancelScan ->
                _state.update { it.copy(scanning = false) }
            BindIntent.CameraDenied ->
                _state.update { it.copy(scanning = false, error = "camera permission denied") }
            is BindIntent.Scanned -> scanQr(intent.qr)
        }
    }

    private fun scanQr(qr: String) {
        if (_state.value.completing) return
        _state.update { it.copy(scanning = false, completing = true, error = "") }
        runOffMain {
            try {
                commands.completeFromQr(qr)
                runOnMain {
                    _state.update {
                        it.copy(
                            bound = true,
                            deviceId = commands.deviceId(),
                            scanning = false,
                            completing = false,
                            error = "",
                        )
                    }
                }
            } catch (error: Exception) {
                runOnMain {
                    _state.update {
                        it.copy(
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
