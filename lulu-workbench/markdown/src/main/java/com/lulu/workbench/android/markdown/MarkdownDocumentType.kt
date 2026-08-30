package com.lulu.workbench.android.markdown

import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Typography
import androidx.compose.runtime.Composable
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.sp

@Composable
internal fun markdownDocumentTypography(): Typography {
    val t = MaterialTheme.typography
    return t.copy(
        headlineSmall = t.titleSmall.copy(fontWeight = FontWeight.SemiBold),
        titleLarge = t.bodyLarge.copy(fontWeight = FontWeight.SemiBold),
        titleMedium = t.bodyMedium.copy(fontWeight = FontWeight.Medium),
        titleSmall = t.bodySmall.copy(fontWeight = FontWeight.Medium),
        bodyLarge = t.bodyMedium,
    )
}

internal val markdownSourceTextStyle = TextStyle(
    fontFamily = FontFamily.Default,
    fontWeight = FontWeight.Normal,
    fontSize = 16.sp,
    lineHeight = 24.sp,
    letterSpacing = 0.15.sp,
)
