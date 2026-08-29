package com.lulu.workbench.android.chat.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
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
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import com.lulu.workbench.android.chat.state.ChatState

@Composable
internal fun ChatDrawer(
    state: ChatState,
    onNewSession: () -> Unit,
    onSelectSession: (String) -> Unit,
    onOpenSettings: () -> Unit,
    modifier: Modifier = Modifier,
) {
    val colors = MaterialTheme.colorScheme
    Column(
        modifier = modifier
            .fillMaxHeight()
            .background(colors.surface)
            .windowInsetsPadding(WindowInsets.safeDrawing)
            .padding(horizontal = 12.dp, vertical = 12.dp),
    ) {
        Text(
            "Chats",
            style = MaterialTheme.typography.titleLarge,
            color = colors.onSurface,
            modifier = Modifier.padding(horizontal = 8.dp, vertical = 8.dp),
        )
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
}

@Composable
private fun DrawerRow(
    label: String,
    selected: Boolean,
    onClick: () -> Unit,
    leading: (@Composable () -> Unit)? = null,
) {
    val colors = MaterialTheme.colorScheme
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .clip(RoundedCornerShape(8.dp))
            .background(if (selected) colors.surfaceVariant else colors.surface)
            .clickable(onClick = onClick)
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
