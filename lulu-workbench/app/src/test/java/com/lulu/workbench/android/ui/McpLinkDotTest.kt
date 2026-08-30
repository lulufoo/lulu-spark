package com.lulu.workbench.android.ui

import androidx.compose.ui.graphics.Color
import com.lulu.workbench.android.ui.theme.StudioOk
import com.lulu.workbench.android.ui.theme.StudioWarn
import com.lulu.workbench.android.wmcp.McpLinkState
import org.junit.Assert.assertEquals
import org.junit.Test

class McpLinkDotTest {
    @Test
    fun descriptionsAreEnglish() {
        assertEquals("MCP unbound", mcpLinkDescription(McpLinkState.Unbound))
        assertEquals("MCP offline", mcpLinkDescription(McpLinkState.Disconnected))
        assertEquals("MCP connected", mcpLinkDescription(McpLinkState.Connected))
    }

    @Test
    fun dotColorsMatchThreeStates() {
        val error = Color(0xFFE06C75)
        assertEquals(StudioWarn, mcpLinkDotColor(McpLinkState.Unbound, error))
        assertEquals(error, mcpLinkDotColor(McpLinkState.Disconnected, error))
        assertEquals(StudioOk, mcpLinkDotColor(McpLinkState.Connected, error))
    }
}
