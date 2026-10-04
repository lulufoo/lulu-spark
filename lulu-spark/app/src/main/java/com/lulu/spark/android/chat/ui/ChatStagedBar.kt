package com.lulu.spark.android.chat.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.widthIn
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.List
import androidx.compose.material.icons.filled.KeyboardArrowDown
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import com.lulu.spark.android.chat.state.ChatStagedItem

@Composable
internal fun ChatStagedBar(
    items: List<ChatStagedItem>,
    onOpen: (String) -> Unit,
    onSeeAll: () -> Unit,
    modifier: Modifier = Modifier,
) {
    if (items.isEmpty()) return
    var open by remember { mutableStateOf(false) }
    val colors = MaterialTheme.colorScheme
    val label = if (items.size == 1) "1 file" else "${items.size} files"
    Column(modifier = modifier.fillMaxWidth()) {
        HorizontalDivider(color = colors.outline.copy(alpha = 0.55f))
        Surface(color = colors.surface) {
            Box {
                Surface(onClick = { open = true }, color = colors.surface) {
                    Row(
                        modifier = Modifier
                            .fillMaxWidth()
                            .padding(horizontal = 16.dp, vertical = 10.dp),
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.spacedBy(10.dp),
                    ) {
                        Icon(
                            Icons.AutoMirrored.Filled.List,
                            contentDescription = "Staged files",
                            modifier = Modifier.size(18.dp),
                            tint = colors.onSurfaceVariant,
                        )
                        Text(
                            label,
                            modifier = Modifier.weight(1f),
                            style = MaterialTheme.typography.labelLarge,
                            color = colors.onSurface,
                        )
                        Icon(
                            Icons.Filled.KeyboardArrowDown,
                            contentDescription = null,
                            modifier = Modifier.size(18.dp),
                            tint = colors.onSurfaceVariant,
                        )
                    }
                }
                DropdownMenu(
                    expanded = open,
                    onDismissRequest = { open = false },
                    modifier = Modifier.widthIn(min = 220.dp),
                    containerColor = colors.surface,
                    shadowElevation = 6.dp,
                ) {
                    items.forEach { item ->
                        DropdownMenuItem(
                            text = {
                                Row(
                                    horizontalArrangement = Arrangement.spacedBy(8.dp),
                                    verticalAlignment = Alignment.CenterVertically,
                                ) {
                                    Text(
                                        item.handle,
                                        style = MaterialTheme.typography.labelLarge,
                                        color = colors.onSurfaceVariant,
                                    )
                                    Text(
                                        item.title,
                                        style = MaterialTheme.typography.bodyLarge,
                                        maxLines = 1,
                                        overflow = TextOverflow.Ellipsis,
                                    )
                                }
                            },
                            onClick = {
                                open = false
                                onOpen(item.id)
                            },
                        )
                    }
                    HorizontalDivider(color = colors.outline.copy(alpha = 0.45f))
                    DropdownMenuItem(
                        text = {
                            Text(
                                "See all",
                                style = MaterialTheme.typography.bodyLarge,
                                color = colors.onSurfaceVariant,
                            )
                        },
                        onClick = {
                            open = false
                            onSeeAll()
                        },
                    )
                }
            }
        }
    }
}
