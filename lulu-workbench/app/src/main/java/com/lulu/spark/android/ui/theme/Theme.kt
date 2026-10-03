package com.lulu.spark.android.ui.theme

import android.app.Activity
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.SideEffect
import androidx.compose.ui.platform.LocalView
import androidx.core.view.WindowCompat

private val StudioDark = darkColorScheme(
    primary = StudioAccent,
    onPrimary = StudioOnAccent,
    primaryContainer = StudioBubble,
    onPrimaryContainer = StudioText,
    secondary = StudioMuted,
    onSecondary = StudioBg,
    background = StudioBg,
    onBackground = StudioText,
    surface = StudioPanel,
    onSurface = StudioText,
    surfaceVariant = StudioRaised,
    onSurfaceVariant = StudioMuted,
    surfaceContainer = StudioPanel,
    surfaceContainerHigh = StudioRaised,
    outline = StudioStroke,
    outlineVariant = StudioStroke,
    error = StudioError,
    onError = StudioOnAccent,
    scrim = StudioScrim,
)

@Composable
fun LuLuWorkbenchTheme(content: @Composable () -> Unit) {
    val view = LocalView.current
    if (!view.isInEditMode) {
        SideEffect {
            val window = (view.context as Activity).window
            WindowCompat.getInsetsController(window, view).apply {
                isAppearanceLightStatusBars = false
                isAppearanceLightNavigationBars = false
            }
        }
    }
    MaterialTheme(
        colorScheme = StudioDark,
        typography = Typography,
        content = content,
    )
}
