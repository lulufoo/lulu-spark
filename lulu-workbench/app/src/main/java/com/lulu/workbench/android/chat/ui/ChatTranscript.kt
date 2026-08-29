package com.lulu.workbench.android.chat.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import com.lulu.workbench.android.agent.session.HistoryTurn
import com.lulu.workbench.android.chat.state.ChatState
import com.lulu.workbench.android.markdown.MarkdownBody

@Composable
internal fun ChatTranscript(
    state: ChatState,
    modifier: Modifier = Modifier,
) {
    if (state.turns.isEmpty() && state.progress.isEmpty()) {
        Box(modifier.fillMaxSize()) {
            Text(
                "Workbench",
                modifier = Modifier.align(Alignment.Center),
                style = MaterialTheme.typography.headlineSmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
        return
    }
    val listState = rememberLazyListState()
    val lastIndex = state.turns.size + if (state.progress.isNotEmpty()) 1 else 0
    LaunchedEffect(lastIndex) {
        if (lastIndex > 0) listState.scrollToItem(lastIndex - 1)
    }
    LazyColumn(
        state = listState,
        modifier = modifier.fillMaxSize(),
        contentPadding = PaddingValues(horizontal = 8.dp, vertical = 16.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        itemsIndexed(
            state.turns,
            key = { index, turn -> "$index:${turn.role}:${turn.content}" },
        ) { _, turn ->
            ChatTurnRow(turn)
        }
        if (state.progress.isNotEmpty()) {
            item(key = "progress") {
                Text(
                    state.progress,
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
        }
    }
}

@Composable
private fun ChatTurnRow(turn: HistoryTurn) {
    val user = turn.role == "user"
    if (user) {
        BoxWithConstraints(modifier = Modifier.fillMaxWidth()) {
            Surface(
                modifier = Modifier
                    .align(Alignment.CenterEnd)
                    .widthIn(max = maxWidth * 0.82f),
                shape = RoundedCornerShape(18.dp),
                color = MaterialTheme.colorScheme.surfaceVariant,
            ) {
                Text(
                    turn.content,
                    modifier = Modifier.padding(horizontal = 14.dp, vertical = 10.dp),
                    style = MaterialTheme.typography.bodyLarge,
                )
            }
        }
    } else {
        MarkdownBody(turn.content, modifier = Modifier.fillMaxWidth())
    }
}
