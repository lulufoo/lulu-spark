package com.lulu.workbench.android.ui.theme

import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.graphics.Color

private val LightColors = lightColorScheme(
    primary = Accent,
    onPrimary = Color.White,
    secondary = InkMuted,
    onSecondary = Color.White,
    background = Paper,
    onBackground = Ink,
    surface = PaperRaised,
    onSurface = Ink,
    surfaceVariant = Chip,
    onSurfaceVariant = InkMuted,
    outline = Color(0xFFE2E2E6),
)

private val DarkColors = darkColorScheme(
    primary = Color.White,
    onPrimary = Ink,
    secondary = Color(0xFFB0B0B4),
    background = Color(0xFF111113),
    onBackground = Color(0xFFF2F2F4),
    surface = Color(0xFF1C1C1F),
    onSurface = Color(0xFFF2F2F4),
    surfaceVariant = Color(0xFF2A2A2E),
    onSurfaceVariant = Color(0xFFB0B0B4),
    outline = Color(0xFF3A3A3E),
)

@Composable
fun LuLuWorkbenchTheme(
    darkTheme: Boolean = isSystemInDarkTheme(),
    content: @Composable () -> Unit,
) {
    MaterialTheme(
        colorScheme = if (darkTheme) DarkColors else LightColors,
        typography = Typography,
        content = content,
    )
}
