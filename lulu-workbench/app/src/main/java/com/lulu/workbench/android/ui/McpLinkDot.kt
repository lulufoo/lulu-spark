package com.lulu.workbench.android.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp
import com.lulu.workbench.android.ui.theme.StudioOk
import com.lulu.workbench.android.ui.theme.StudioWarn
import com.lulu.workbench.android.wmcp.McpLinkState

@Composable
fun McpLinkDot(
    state: McpLinkState,
    modifier: Modifier = Modifier,
) {
    Box(
        modifier = modifier
            .size(8.dp)
            .clip(CircleShape)
            .background(mcpLinkDotColor(state, MaterialTheme.colorScheme.error))
            .semantics { contentDescription = mcpLinkDescription(state) },
    )
}

internal fun mcpLinkDescription(state: McpLinkState): String =
    when (state) {
        McpLinkState.Unbound -> "MCP unbound"
        McpLinkState.Disconnected -> "MCP offline"
        McpLinkState.Connected -> "MCP connected"
    }

internal fun mcpLinkDotColor(
    state: McpLinkState,
    error: Color,
): Color =
    when (state) {
        McpLinkState.Unbound -> StudioWarn
        McpLinkState.Disconnected -> error
        McpLinkState.Connected -> StudioOk
    }
