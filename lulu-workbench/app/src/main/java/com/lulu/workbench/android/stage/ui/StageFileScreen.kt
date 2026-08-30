package com.lulu.workbench.android.stage.ui

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalFocusManager
import androidx.compose.ui.platform.LocalSoftwareKeyboardController
import androidx.compose.ui.unit.dp
import com.lulu.workbench.android.agent.tools.stage.StagedItem
import com.lulu.workbench.android.markdown.MarkdownPane
import com.lulu.workbench.android.markdown.MarkdownPaneMode

@Composable
internal fun StageFileScreen(
    item: StagedItem,
    onBack: () -> Unit,
    onSave: (title: String, body: String) -> Unit,
    onDelete: () -> Unit,
    modifier: Modifier = Modifier,
) {
    var title by remember(item.id, item.title) { mutableStateOf(item.title) }
    var body by remember(item.id, item.body) { mutableStateOf(item.body) }
    var mode by remember(item.id) { mutableStateOf(MarkdownPaneMode.Preview) }
    var confirmDelete by remember(item.id) { mutableStateOf(false) }
    val dirty = title != item.title || body != item.body
    val colors = MaterialTheme.colorScheme
    val focusManager = LocalFocusManager.current
    val keyboard = LocalSoftwareKeyboardController.current
    fun dismissIme() {
        focusManager.clearFocus(force = true)
        keyboard?.hide()
    }
    Surface(
        modifier = modifier
            .fillMaxSize()
            .dismissImeOnBackgroundTap { dismissIme() },
        color = colors.background,
    ) {
        Column(modifier = Modifier.fillMaxSize()) {
            StageHeader(
                title = title.ifBlank { item.title },
                subtitle = fileSubtitle(item),
                onBack = onBack,
                action = {
                    if (mode == MarkdownPaneMode.Edit) {
                        TextButton(
                            onClick = {
                                onSave(title, body)
                                mode = MarkdownPaneMode.Preview
                                dismissIme()
                            },
                            enabled = dirty,
                        ) {
                            Text("Save", color = if (dirty) colors.onSurface else colors.onSurfaceVariant)
                        }
                    } else {
                        TextButton(onClick = { confirmDelete = true }) {
                            Text("Delete", color = colors.error)
                        }
                    }
                },
            )
            HorizontalDivider(color = colors.outline.copy(alpha = 0.55f))
            Column(
                modifier = Modifier
                    .weight(1f)
                    .imePadding()
                    .padding(horizontal = 16.dp, vertical = 16.dp),
            ) {
                if (mode == MarkdownPaneMode.Edit) {
                    OutlinedTextField(
                        value = title,
                        onValueChange = { title = it },
                        modifier = Modifier
                            .fillMaxWidth()
                            .padding(bottom = 12.dp),
                        singleLine = true,
                        textStyle = MaterialTheme.typography.bodyLarge,
                        label = { Text("Title", style = MaterialTheme.typography.bodySmall) },
                        colors = stageFieldColors(),
                    )
                }
                MarkdownPane(
                    source = body,
                    onSourceChange = { body = it },
                    mode = mode,
                    onModeChange = {
                        mode = it
                        if (it == MarkdownPaneMode.Preview) dismissIme()
                    },
                    modifier = Modifier
                        .weight(1f)
                        .fillMaxWidth(),
                )
            }
        }
        if (confirmDelete) {
            AlertDialog(
                onDismissRequest = { confirmDelete = false },
                title = { Text("Delete draft") },
                text = { Text("Delete “${item.title}”? This cannot be undone.") },
                confirmButton = {
                    TextButton(
                        onClick = {
                            confirmDelete = false
                            onDelete()
                        },
                    ) {
                        Text("Delete", color = colors.error)
                    }
                },
                dismissButton = {
                    TextButton(onClick = { confirmDelete = false }) {
                        Text("Cancel", color = colors.onSurfaceVariant)
                    }
                },
                containerColor = colors.surface,
            )
        }
    }
}

private fun fileSubtitle(item: StagedItem): String {
    val rest = item.sourceSessionTitle
    return if (rest.isBlank()) item.handle else "${item.handle} · $rest"
}
