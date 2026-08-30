package com.lulu.workbench.android.bind.state

import com.lulu.workbench.android.wmcp.McpLinkState

data class BindState(
    val bound: Boolean = false,
    val deviceId: String = "",
    val scanning: Boolean = false,
    val completing: Boolean = false,
    val error: String = "",
    val mcpLink: McpLinkState = McpLinkState.Unbound,
)

sealed class BindIntent {
    data object Query : BindIntent()

    data object StartScan : BindIntent()

    data object CancelScan : BindIntent()

    data object CameraDenied : BindIntent()

    data class Scanned(val qr: String) : BindIntent()
}
