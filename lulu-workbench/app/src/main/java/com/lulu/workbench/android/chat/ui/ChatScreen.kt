package com.lulu.workbench.android.chat.ui

import androidx.activity.compose.BackHandler
import androidx.compose.animation.core.animateDpAsState
import androidx.compose.animation.core.tween
import androidx.compose.foundation.clickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.Menu
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
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
import androidx.compose.ui.draw.shadow
import androidx.compose.ui.unit.dp
import com.lulu.workbench.android.chat.state.ChatIntent
import com.lulu.workbench.android.chat.state.ChatState
import com.lulu.workbench.android.chat.state.ChatStore

@Composable
fun ChatScreen(
    store: ChatStore,
    onOpenSettings: () -> Unit,
    modifier: Modifier = Modifier,
) {
    var drawerOpen by remember { mutableStateOf(false) }
    fun closeDrawer() {
        drawerOpen = false
    }
    BackHandler(enabled = drawerOpen) { closeDrawer() }
    BoxWithConstraints(modifier = modifier.fillMaxSize()) {
        val drawerWidth = maxWidth * 0.78f
        val offset by animateDpAsState(
            targetValue = if (drawerOpen) drawerWidth else 0.dp,
            animationSpec = tween(durationMillis = 280),
            label = "chatSlide",
        )
        ChatDrawer(
            state = store.state,
            onNewSession = {
                store.dispatch(ChatIntent.NewSession)
                closeDrawer()
            },
            onSelectSession = { id ->
                store.dispatch(ChatIntent.SelectSession(id))
                closeDrawer()
            },
            onOpenSettings = {
                closeDrawer()
                onOpenSettings()
            },
            modifier = Modifier
                .width(drawerWidth)
                .fillMaxHeight(),
        )
        Surface(
            modifier = Modifier
                .fillMaxSize()
                .offset(x = offset)
                .shadow(if (drawerOpen) 12.dp else 0.dp),
            color = MaterialTheme.colorScheme.background,
        ) {
            ChatPane(
                state = store.state,
                onOpenDrawer = { drawerOpen = true },
                onNewSession = { store.dispatch(ChatIntent.NewSession) },
                onSend = { text -> store.dispatch(ChatIntent.Send(text)) },
            )
            if (drawerOpen) {
                Box(
                    modifier = Modifier
                        .fillMaxSize()
                        .clickable(
                            interactionSource = remember { MutableInteractionSource() },
                            indication = null,
                        ) { closeDrawer() },
                )
            }
        }
    }
}

@Composable
private fun ChatPane(
    state: ChatState,
    onOpenDrawer: () -> Unit,
    onNewSession: () -> Unit,
    onSend: (String) -> Unit,
) {
    var draft by remember { mutableStateOf("") }
    val sendEnabled = !state.inFlight && draft.isNotBlank()
    val title = state.sessions.firstOrNull { it.id == state.sessionId }?.title ?: "Workbench"
    fun submit() {
        if (!sendEnabled) return
        val text = draft
        draft = ""
        onSend(text)
    }
    Column(
        modifier = Modifier
            .fillMaxSize()
            .imePadding()
            .padding(horizontal = 12.dp, vertical = 4.dp),
    ) {
        Row(
            modifier = Modifier.fillMaxWidth(),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            IconButton(onClick = onOpenDrawer) {
                Icon(Icons.Filled.Menu, contentDescription = "Chats")
            }
            Text(
                text = title,
                modifier = Modifier.weight(1f),
                style = MaterialTheme.typography.titleMedium,
                maxLines = 1,
            )
            IconButton(onClick = onNewSession, enabled = !state.inFlight) {
                Icon(Icons.Filled.Add, contentDescription = "New chat")
            }
        }
        ChatTranscript(
            state = state,
            modifier = Modifier
                .weight(1f)
                .fillMaxWidth(),
        )
        ChatComposer(
            draft = draft,
            onDraftChange = { draft = it },
            enabled = !state.inFlight,
            sendEnabled = sendEnabled,
            onSend = { submit() },
            modifier = Modifier.padding(start = 4.dp, end = 4.dp, bottom = 8.dp),
        )
    }
}
