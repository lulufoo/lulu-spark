package com.lulu.spark.android.ui

import androidx.compose.ui.graphics.Color
import com.lulu.spark.android.ui.theme.StudioOk
import com.lulu.spark.android.ui.theme.StudioWarn
import com.lulu.spark.android.wmcp.McpLinkState
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
