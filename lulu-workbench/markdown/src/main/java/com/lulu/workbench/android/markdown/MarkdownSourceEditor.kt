package com.lulu.workbench.android.markdown

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.unit.dp

@Composable
internal fun MarkdownSourceEditor(
    source: String,
    onSourceChange: (String) -> Unit,
    modifier: Modifier = Modifier,
    placeholder: String = "Markdown…",
) {
    val colors = MaterialTheme.colorScheme
    BasicTextField(
        value = source,
        onValueChange = onSourceChange,
        modifier = modifier
            .fillMaxSize()
            .verticalScroll(rememberScrollState())
            .padding(16.dp),
        textStyle = markdownSourceTextStyle.copy(color = colors.onSurface),
        cursorBrush = SolidColor(colors.onSurfaceVariant),
        decorationBox = { inner ->
            Box(modifier = Modifier.fillMaxWidth()) {
                if (source.isEmpty()) {
                    Text(
                        placeholder,
                        style = markdownSourceTextStyle.copy(color = colors.onSurfaceVariant),
                    )
                }
                inner()
            }
        },
    )
}
