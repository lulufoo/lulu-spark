package com.lulu.spark.android.chat.state

import com.lulu.spark.android.agent.session.HistoryTurn
import com.lulu.spark.android.wmcp.McpLinkState

data class ChatSessionItem(
    val id: String,
    val title: String,
)

enum class VoicePhase {
    Idle,
    Recording,
    Recognizing,
}

data class ChatState(
    val sessionId: String? = null,
    val progress: String = "",
    val lastReply: String = "",
    val inFlight: Boolean = false,
    val sessions: List<ChatSessionItem> = emptyList(),
    val turns: List<HistoryTurn> = emptyList(),
    val voicePhase: VoicePhase = VoicePhase.Idle,
    val voiceHint: String = "",
    val asrConfigured: Boolean = false,
    val mcpLink: McpLinkState = McpLinkState.Unbound,
    val stagedThisChat: List<ChatStagedItem> = emptyList(),
)

data class ChatStagedItem(
    val id: String,
    val handle: String,
    val title: String,
)

sealed class ChatIntent {
    data object NewSession : ChatIntent()

    data class SelectSession(val id: String) : ChatIntent()

    data class DeleteSession(val id: String) : ChatIntent()

    data class Send(val text: String) : ChatIntent()

    data object VoicePress : ChatIntent()

    data class VoiceRelease(val cancel: Boolean) : ChatIntent()

    data object MicDenied : ChatIntent()

    data object RefreshAsr : ChatIntent()

    data object RefreshStaged : ChatIntent()

    data object ClearVoiceHint : ChatIntent()
}

internal fun lastAssistant(turns: List<HistoryTurn>): String =
    turns.lastOrNull { it.role == "assistant" }?.content.orEmpty()

internal fun visibleTurns(turns: List<HistoryTurn>): List<HistoryTurn> =
    turns.filter { it.role == "user" || it.role == "assistant" }
