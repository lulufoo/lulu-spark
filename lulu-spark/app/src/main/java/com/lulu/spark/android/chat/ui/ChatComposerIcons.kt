package com.lulu.spark.android.chat.ui

import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.graphics.vector.path
import androidx.compose.ui.unit.dp

internal val ComposerMic: ImageVector by lazy {
    ImageVector.Builder(
        name = "ComposerMic",
        defaultWidth = 24.dp,
        defaultHeight = 24.dp,
        viewportWidth = 24f,
        viewportHeight = 24f,
    ).apply {
        path(fill = SolidColor(Color.Black)) {
            moveTo(12f, 14f)
            curveToRelative(1.66f, 0f, 2.99f, -1.34f, 2.99f, -3f)
            lineTo(15f, 5f)
            curveToRelative(0f, -1.66f, -1.34f, -3f, -3f, -3f)
            reflectiveCurveTo(9f, 3.34f, 9f, 5f)
            verticalLineToRelative(6f)
            curveToRelative(0f, 1.66f, 1.34f, 3f, 3f, 3f)
            close()
            moveTo(17.3f, 11f)
            curveToRelative(0f, 3f, -2.54f, 5.1f, -5.3f, 5.1f)
            reflectiveCurveTo(6.7f, 14f, 6.7f, 11f)
            lineTo(5f, 11f)
            curveToRelative(0f, 3.41f, 2.72f, 6.23f, 6f, 6.72f)
            lineTo(11f, 21f)
            horizontalLineToRelative(2f)
            verticalLineToRelative(-3.28f)
            curveToRelative(3.28f, -0.48f, 6f, -3.3f, 6f, -6.72f)
            horizontalLineToRelative(-1.7f)
            close()
        }
    }.build()
}

internal val ComposerKeyboard: ImageVector by lazy {
    ImageVector.Builder(
        name = "ComposerKeyboard",
        defaultWidth = 24.dp,
        defaultHeight = 24.dp,
        viewportWidth = 24f,
        viewportHeight = 24f,
    ).apply {
        path(fill = SolidColor(Color.Black)) {
            moveTo(20f, 5f)
            lineTo(4f, 5f)
            curveToRelative(-1.1f, 0f, -1.99f, 0.9f, -1.99f, 2f)
            lineTo(2f, 17f)
            curveToRelative(0f, 1.1f, 0.9f, 2f, 2f, 2f)
            horizontalLineToRelative(16f)
            curveToRelative(1.1f, 0f, 2f, -0.9f, 2f, -2f)
            lineTo(22f, 7f)
            curveToRelative(0f, -1.1f, -0.9f, -2f, -2f, -2f)
            close()
            moveTo(11f, 8f)
            horizontalLineToRelative(2f)
            verticalLineToRelative(2f)
            horizontalLineToRelative(-2f)
            lineTo(11f, 8f)
            close()
            moveTo(11f, 11f)
            horizontalLineToRelative(2f)
            verticalLineToRelative(2f)
            horizontalLineToRelative(-2f)
            verticalLineToRelative(-2f)
            close()
            moveTo(8f, 8f)
            horizontalLineToRelative(2f)
            verticalLineToRelative(2f)
            lineTo(8f, 10f)
            lineTo(8f, 8f)
            close()
            moveTo(8f, 11f)
            horizontalLineToRelative(2f)
            verticalLineToRelative(2f)
            lineTo(8f, 13f)
            verticalLineToRelative(-2f)
            close()
            moveTo(7f, 13f)
            lineTo(5f, 13f)
            verticalLineToRelative(-2f)
            horizontalLineToRelative(2f)
            verticalLineToRelative(2f)
            close()
            moveTo(7f, 10f)
            lineTo(5f, 10f)
            lineTo(5f, 8f)
            horizontalLineToRelative(2f)
            verticalLineToRelative(2f)
            close()
            moveTo(16f, 17f)
            lineTo(8f, 17f)
            verticalLineToRelative(-2f)
            horizontalLineToRelative(8f)
            verticalLineToRelative(2f)
            close()
            moveTo(16f, 13f)
            horizontalLineToRelative(-2f)
            verticalLineToRelative(-2f)
            horizontalLineToRelative(2f)
            verticalLineToRelative(2f)
            close()
            moveTo(16f, 10f)
            horizontalLineToRelative(-2f)
            lineTo(14f, 8f)
            horizontalLineToRelative(2f)
            verticalLineToRelative(2f)
            close()
            moveTo(19f, 13f)
            horizontalLineToRelative(-2f)
            verticalLineToRelative(-2f)
            horizontalLineToRelative(2f)
            verticalLineToRelative(2f)
            close()
            moveTo(19f, 10f)
            horizontalLineToRelative(-2f)
            lineTo(17f, 8f)
            horizontalLineToRelative(2f)
            verticalLineToRelative(2f)
            close()
        }
    }.build()
}
