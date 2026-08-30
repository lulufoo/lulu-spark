package com.lulu.workbench.android.stage.ui

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.KeyboardArrowRight
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import com.lulu.workbench.android.agent.tools.stage.StagedItem
import com.lulu.workbench.android.stage.state.StageState

@Composable
internal fun StageListScreen(
    state: StageState,
    onBack: () -> Unit,
    onOpen: (String) -> Unit,
    modifier: Modifier = Modifier,
) {
    val colors = MaterialTheme.colorScheme
    val thisChat = remember(state.items, state.currentSessionId) {
        state.items.filter { it.sourceSessionId == state.currentSessionId }
    }
    val other = remember(state.items, state.currentSessionId) {
        state.items.filter { it.sourceSessionId != state.currentSessionId }
    }
    Surface(modifier = modifier.fillMaxSize(), color = colors.background) {
        Column(modifier = Modifier.fillMaxSize()) {
            StageHeader(
                title = "Staged",
                subtitle = "Markdown drafts on this phone",
                onBack = onBack,
            )
            HorizontalDivider(color = colors.outline.copy(alpha = 0.55f))
            if (state.items.isEmpty()) {
                Box(modifier = Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                    Column(horizontalAlignment = Alignment.CenterHorizontally) {
                        Text(
                            "Nothing staged",
                            style = MaterialTheme.typography.headlineSmall,
                            color = colors.onSurfaceVariant.copy(alpha = 0.7f),
                        )
                        Text(
                            "Ask the assistant to stage a draft.",
                            modifier = Modifier.padding(top = 8.dp),
                            style = MaterialTheme.typography.bodyMedium,
                            color = colors.onSurfaceVariant,
                        )
                    }
                }
            } else {
                LazyColumn(
                    modifier = Modifier.fillMaxSize(),
                    contentPadding = PaddingValues(horizontal = 16.dp, vertical = 16.dp),
                ) {
                    if (thisChat.isNotEmpty()) {
                        item(key = "this-chat") {
                            StageGroup(
                                label = "This chat",
                                items = thisChat,
                                subtitleOf = { null },
                                onOpen = onOpen,
                            )
                        }
                    }
                    if (thisChat.isNotEmpty() && other.isNotEmpty()) {
                        item(key = "gap") { Spacer(Modifier.height(20.dp)) }
                    }
                    if (other.isNotEmpty()) {
                        item(key = "other") {
                            StageGroup(
                                label = "Other chats",
                                items = other,
                                subtitleOf = { it.sourceSessionTitle.ifBlank { "Other chat" } },
                                onOpen = onOpen,
                            )
                        }
                    }
                }
            }
        }
    }
}

@Composable
private fun StageGroup(
    label: String,
    items: List<StagedItem>,
    subtitleOf: (StagedItem) -> String?,
    onOpen: (String) -> Unit,
) {
    val colors = MaterialTheme.colorScheme
    StageSectionLabel(label)
    StageCard {
        items.forEachIndexed { index, item ->
            if (index > 0) {
                HorizontalDivider(
                    modifier = Modifier.padding(start = 16.dp),
                    color = colors.outline.copy(alpha = 0.4f),
                )
            }
            StageRow(
                handle = item.handle,
                title = item.title,
                subtitle = subtitleOf(item),
                onClick = { onOpen(item.id) },
            )
        }
    }
}

@Composable
private fun StageRow(
    handle: String,
    title: String,
    subtitle: String?,
    onClick: () -> Unit,
) {
    val colors = MaterialTheme.colorScheme
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .clickable(onClick = onClick)
            .padding(horizontal = 16.dp, vertical = 14.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(10.dp),
    ) {
        Text(
            handle,
            style = MaterialTheme.typography.labelLarge,
            color = colors.onSurfaceVariant,
        )
        Column(modifier = Modifier.weight(1f)) {
            Text(
                title,
                style = MaterialTheme.typography.titleMedium,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
            )
            if (!subtitle.isNullOrBlank()) {
                Text(
                    subtitle,
                    modifier = Modifier.padding(top = 4.dp),
                    style = MaterialTheme.typography.bodySmall,
                    color = colors.onSurfaceVariant,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                )
            }
        }
        Icon(
            Icons.AutoMirrored.Filled.KeyboardArrowRight,
            contentDescription = null,
            tint = colors.onSurfaceVariant,
        )
    }
}
