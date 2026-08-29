package com.lulu.workbench.android.chat.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Settings
import androidx.compose.material3.Button
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TextField
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.input.ImeAction
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
    ChatPane(
        state = store.state,
        onOpenSettings = onOpenSettings,
        onNewSession = { store.dispatch(ChatIntent.NewSession) },
        onSend = { text -> store.dispatch(ChatIntent.Send(text)) },
        modifier = modifier,
    )
}

@Composable
private fun ChatPane(
    state: ChatState,
    onOpenSettings: () -> Unit,
    onNewSession: () -> Unit,
    onSend: (String) -> Unit,
    modifier: Modifier = Modifier,
) {
    var draft by remember { mutableStateOf("") }
    val sendEnabled = !state.inFlight && draft.isNotBlank()
    fun submit() {
        if (!sendEnabled) return
        val text = draft
        draft = ""
        onSend(text)
    }
    Column(modifier = modifier.fillMaxSize().padding(16.dp)) {
        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.SpaceBetween,
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Text("Chat")
            IconButton(onClick = onOpenSettings) {
                Icon(Icons.Filled.Settings, contentDescription = "Settings")
            }
        }
        Text("session: ${state.sessionId ?: "-"}")
        Spacer(modifier = Modifier.weight(1f))
        Text("reply: ${state.lastReply.ifEmpty { "-" }}")
        if (state.progress.isNotEmpty()) {
            Text(state.progress)
        }
        Row(
            modifier = Modifier.fillMaxWidth(),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            TextField(
                value = draft,
                onValueChange = { draft = it },
                modifier = Modifier.weight(1f).padding(end = 8.dp),
                enabled = !state.inFlight,
                singleLine = true,
                placeholder = { Text("Message…") },
                keyboardOptions = KeyboardOptions(imeAction = ImeAction.Send),
                keyboardActions = KeyboardActions(onSend = { submit() }),
            )
            Button(onClick = { submit() }, enabled = sendEnabled) { Text("Send") }
        }
        TextButton(onClick = onNewSession) { Text("New session") }
    }
}
