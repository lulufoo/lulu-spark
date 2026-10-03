package com.lulu.workbench.android.chat.ui

import androidx.compose.foundation.ExperimentalFoundationApi
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.combinedClickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.safeDrawing
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.Settings
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import com.lulu.workbench.android.chat.state.ChatSessionItem
import com.lulu.workbench.android.chat.state.ChatState
import com.lulu.workbench.android.ui.McpLinkDot

@Composable
internal fun ChatDrawer(
    state: ChatState,
    onNewSession: () -> Unit,
    onSelectSession: (String) -> Unit,
    onDeleteSession: (String) -> Unit,
    onOpenSettings: () -> Unit,
    modifier: Modifier = Modifier,
) {
    val colors = MaterialTheme.colorScheme
    var pendingDelete by remember { mutableStateOf<ChatSessionItem?>(null) }
    Column(
        modifier = modifier
            .fillMaxHeight()
            .background(colors.surface)
            .windowInsetsPadding(WindowInsets.safeDrawing)
            .padding(horizontal = 12.dp, vertical = 12.dp),
    ) {
        Row(
            modifier = Modifier.padding(horizontal = 8.dp, vertical = 8.dp),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            Text(
                "Lulu Spark",
                style = MaterialTheme.typography.titleLarge,
                color = colors.onSurface,
            )
            McpLinkDot(state.mcpLink)
        }
        Spacer(modifier = Modifier.height(8.dp))
        DrawerRow(
            label = "New chat",
            selected = false,
            leading = {
                Icon(Icons.Filled.Add, contentDescription = null)
            },
            onClick = onNewSession,
        )
        Spacer(modifier = Modifier.height(12.dp))
        if (state.sessions.isEmpty()) {
            Text(
                "No chats yet",
                style = MaterialTheme.typography.bodyMedium,
                color = colors.onSurfaceVariant,
                modifier = Modifier.padding(horizontal = 8.dp, vertical = 8.dp),
            )
        } else {
            LazyColumn(modifier = Modifier.weight(1f, fill = true)) {
                items(state.sessions, key = { it.id }) { item ->
                    DrawerRow(
                        label = item.title,
                        selected = item.id == state.sessionId,
                        onClick = { onSelectSession(item.id) },
                        onLongClick = { pendingDelete = item },
                    )
                }
            }
        }
        if (state.sessions.isEmpty()) {
            Spacer(modifier = Modifier.weight(1f))
        }
        HorizontalDivider(color = colors.outline.copy(alpha = 0.55f))
        Spacer(modifier = Modifier.height(8.dp))
        DrawerRow(
            label = "Settings",
            selected = false,
            leading = {
                Icon(Icons.Filled.Settings, contentDescription = null)
            },
            onClick = onOpenSettings,
        )
    }
    val doomed = pendingDelete
    if (doomed != null) {
        AlertDialog(
            onDismissRequest = { pendingDelete = null },
            title = { Text("Delete chat") },
            text = { Text("Delete “${doomed.title}”? This cannot be undone.") },
            confirmButton = {
                TextButton(
                    onClick = {
                        onDeleteSession(doomed.id)
                        pendingDelete = null
                    },
                ) {
                    Text("Delete", color = colors.error)
                }
            },
            dismissButton = {
                TextButton(onClick = { pendingDelete = null }) {
                    Text("Cancel", color = colors.onSurfaceVariant)
                }
            },
            containerColor = colors.surface,
        )
    }
}

@OptIn(ExperimentalFoundationApi::class)
@Composable
private fun DrawerRow(
    label: String,
    selected: Boolean,
    onClick: () -> Unit,
    onLongClick: (() -> Unit)? = null,
    leading: (@Composable () -> Unit)? = null,
) {
    val colors = MaterialTheme.colorScheme
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .clip(RoundedCornerShape(8.dp))
            .background(if (selected) colors.surfaceVariant else colors.surface)
            .then(
                if (onLongClick == null) {
                    Modifier.clickable(onClick = onClick)
                } else {
                    Modifier.combinedClickable(onClick = onClick, onLongClick = onLongClick)
                },
            )
            .padding(horizontal = 10.dp, vertical = 10.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(10.dp),
    ) {
        if (leading != null) leading()
        Text(
            text = label,
            style = MaterialTheme.typography.bodyLarge,
            color = if (selected) colors.onSurface else colors.onSurfaceVariant,
            maxLines = 1,
            overflow = TextOverflow.Ellipsis,
        )
    }
}
