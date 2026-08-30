package com.lulu.workbench.android.markdown

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import com.lulu.workbench.android.markdown.render.MdDocument

private val DocumentPadH = 16.dp
private val DocumentPadV = 12.dp

enum class MarkdownPaneMode {
    Preview,
    Edit,
}

/** Document surface: exclusive Preview / Edit, same as the Mac md pane. */
@Composable
fun MarkdownPane(
    source: String,
    onSourceChange: (String) -> Unit,
    mode: MarkdownPaneMode,
    onModeChange: (MarkdownPaneMode) -> Unit,
    modifier: Modifier = Modifier,
    placeholder: String = "Markdown…",
) {
    val colors = MaterialTheme.colorScheme
    Column(modifier = modifier.fillMaxSize()) {
        Row(
            modifier = Modifier.padding(horizontal = 12.dp, vertical = 8.dp),
            horizontalArrangement = Arrangement.spacedBy(4.dp),
        ) {
            ModeTab(
                label = "Preview",
                selected = mode == MarkdownPaneMode.Preview,
                onClick = { onModeChange(MarkdownPaneMode.Preview) },
            )
            ModeTab(
                label = "Edit",
                selected = mode == MarkdownPaneMode.Edit,
                onClick = { onModeChange(MarkdownPaneMode.Edit) },
            )
        }
        HorizontalDivider(color = colors.outline.copy(alpha = 0.45f))
        when (mode) {
            MarkdownPaneMode.Preview -> MarkdownPreview(source)
            MarkdownPaneMode.Edit -> MarkdownSourceEditor(
                source = source,
                onSourceChange = onSourceChange,
                placeholder = placeholder,
            )
        }
    }
}

@Composable
private fun MarkdownPreview(source: String) {
    val colors = MaterialTheme.colorScheme
    if (source.isBlank()) {
        Box(
            modifier = Modifier
                .fillMaxSize()
                .padding(horizontal = DocumentPadH, vertical = DocumentPadV),
            contentAlignment = Alignment.TopStart,
        ) {
            Text(
                "Empty draft",
                style = MaterialTheme.typography.bodyMedium,
                color = colors.onSurfaceVariant,
            )
        }
    } else {
        MaterialTheme(colorScheme = colors, typography = markdownDocumentTypography()) {
            MdDocument(
                source,
                modifier = Modifier
                    .fillMaxSize()
                    .verticalScroll(rememberScrollState())
                    .padding(horizontal = DocumentPadH, vertical = DocumentPadV),
            )
        }
    }
}

@Composable
private fun ModeTab(
    label: String,
    selected: Boolean,
    onClick: () -> Unit,
) {
    val colors = MaterialTheme.colorScheme
    Surface(
        onClick = onClick,
        shape = RoundedCornerShape(8.dp),
        color = if (selected) colors.surfaceVariant else colors.surface,
    ) {
        Text(
            label,
            modifier = Modifier.padding(horizontal = 12.dp, vertical = 6.dp),
            style = MaterialTheme.typography.labelLarge,
            color = if (selected) colors.onSurface else colors.onSurfaceVariant,
        )
    }
}
