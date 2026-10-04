package com.lulu.spark.android.chat.ui

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.statusBars
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.Menu
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalFocusManager
import androidx.compose.ui.platform.LocalSoftwareKeyboardController
import com.lulu.spark.android.chat.state.ChatState

@Composable
internal fun ChatPane(
    state: ChatState,
    transcriptReady: Boolean,
    onOpenDrawer: () -> Unit,
    onNewSession: () -> Unit,
    onOpenStaged: (String) -> Unit,
    onOpenStagedAll: () -> Unit,
    onSend: (String) -> Unit,
    onVoicePress: () -> Unit,
    onVoiceRelease: (Boolean) -> Unit,
    onVoiceHintShown: () -> Unit,
) {
    var draft by remember { mutableStateOf("") }
    val focusManager = LocalFocusManager.current
    val keyboard = LocalSoftwareKeyboardController.current
    fun dismissIme() {
        focusManager.clearFocus(force = true)
        keyboard?.hide()
    }
    val sendEnabled = !state.inFlight && draft.isNotBlank()
    val title = state.sessions.firstOrNull { it.id == state.sessionId }?.title ?: "Lulu Spark"
    fun submit() {
        if (!sendEnabled) return
        val text = draft
        draft = ""
        onSend(text)
    }
    Column(modifier = Modifier.fillMaxSize()) {
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .windowInsetsPadding(WindowInsets.statusBars)
                .dismissImeOnTap { dismissIme() },
            verticalAlignment = Alignment.CenterVertically,
        ) {
            IconButton(onClick = onOpenDrawer) {
                Icon(Icons.Filled.Menu, contentDescription = "Chats")
            }
            Text(
                text = title,
                modifier = Modifier.weight(1f),
                style = MaterialTheme.typography.titleMedium,
                color = MaterialTheme.colorScheme.onSurface,
                maxLines = 1,
            )
            IconButton(onClick = onNewSession, enabled = !state.inFlight) {
                Icon(Icons.Filled.Add, contentDescription = "New chat")
            }
        }
        HorizontalDivider(color = MaterialTheme.colorScheme.outline.copy(alpha = 0.55f))
        ChatTranscript(
            state = state,
            ready = transcriptReady,
            modifier = Modifier
                .weight(1f)
                .fillMaxWidth()
                .dismissImeOnTap { dismissIme() },
        )
        ChatStagedBar(
            items = state.stagedThisChat,
            onOpen = onOpenStaged,
            onSeeAll = onOpenStagedAll,
        )
        ChatComposer(
            draft = draft,
            onDraftChange = { draft = it },
            sendEnabled = sendEnabled,
            onSend = { submit() },
            voicePhase = state.voicePhase,
            voiceHint = state.voiceHint,
            voiceEnabled = voiceHoldEnabled(
                inFlight = state.inFlight,
                phase = state.voicePhase,
                asrConfigured = state.asrConfigured,
            ),
            asrConfigured = state.asrConfigured,
            onVoicePress = onVoicePress,
            onVoiceRelease = onVoiceRelease,
            onVoiceHintShown = onVoiceHintShown,
            modifier = Modifier.chatComposerImePadding(),
        )
    }
}
