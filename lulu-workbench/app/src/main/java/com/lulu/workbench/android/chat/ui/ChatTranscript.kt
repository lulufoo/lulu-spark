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
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import com.lulu.workbench.android.agent.session.HistoryTurn
import com.lulu.workbench.android.chat.state.ChatState
import com.lulu.workbench.android.markdown.MarkdownBody

private val TranscriptPadH = 20.dp
private val TranscriptPadV = 20.dp
private val TurnGap = 10.dp
private val RoleChangeExtra = 8.dp
private val AssistantEndGutter = 32.dp
private val UserMaxFraction = 0.78f

@Composable
internal fun ChatTranscript(
    state: ChatState,
    ready: Boolean = true,
    modifier: Modifier = Modifier,
) {
    if (!ready) {
        Box(modifier.fillMaxSize())
        return
    }
    if (state.turns.isEmpty() && state.progress.isEmpty()) {
        Box(modifier.fillMaxSize()) {
            Text(
                "Workbench",
                modifier = Modifier.align(Alignment.Center),
                style = MaterialTheme.typography.headlineSmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = 0.7f),
            )
        }
        return
    }
    val turns = state.turns
    val lastIndex = turns.size + if (state.progress.isNotEmpty()) 1 else 0
    val listState = rememberLazyListState()
    LaunchedEffect(state.sessionId, lastIndex) {
        if (lastIndex > 0) listState.scrollToItem(0)
    }
    LazyColumn(
        state = listState,
        modifier = modifier.fillMaxSize(),
        reverseLayout = true,
        contentPadding = PaddingValues(horizontal = TranscriptPadH, vertical = TranscriptPadV),
        verticalArrangement = Arrangement.spacedBy(TurnGap),
    ) {
        if (state.progress.isNotEmpty()) {
            item(key = "progress") {
                Text(
                    state.progress,
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(end = AssistantEndGutter, top = RoleChangeExtra),
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
        }
        items(
            count = turns.size,
            key = { reverseIndex ->
                val index = turns.lastIndex - reverseIndex
                val turn = turns[index]
                "$index:${turn.role}:${turn.content}"
            },
        ) { reverseIndex ->
            val index = turns.lastIndex - reverseIndex
            val turn = turns[index]
            val prevRole = turns.getOrNull(index - 1)?.role
            val extraTop = if (prevRole != null && prevRole != turn.role) RoleChangeExtra else 0.dp
            ChatTurnRow(turn, extraTop)
        }
    }
}

@Composable
private fun ChatTurnRow(turn: HistoryTurn, extraTop: Dp) {
    val user = turn.role == "user"
    if (user) {
        BoxWithConstraints(
            modifier = Modifier
                .fillMaxWidth()
                .padding(top = extraTop),
        ) {
            Surface(
                modifier = Modifier
                    .align(Alignment.CenterEnd)
                    .widthIn(max = maxWidth * UserMaxFraction),
                shape = RoundedCornerShape(18.dp, 18.dp, 6.dp, 18.dp),
                color = MaterialTheme.colorScheme.primaryContainer,
            ) {
                Text(
                    turn.content,
                    modifier = Modifier.padding(horizontal = 14.dp, vertical = 10.dp),
                    style = MaterialTheme.typography.bodyLarge,
                    color = MaterialTheme.colorScheme.onPrimaryContainer,
                )
            }
        }
    } else {
        MarkdownBody(
            turn.content,
            modifier = Modifier
                .fillMaxWidth()
                .padding(top = extraTop, end = AssistantEndGutter),
        )
    }
}
