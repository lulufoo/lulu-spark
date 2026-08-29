package com.lulu.workbench.android.chat.state

import com.lulu.workbench.android.agent.session.HistoryTurn

data class ChatSessionItem(
    val id: String,
    val title: String,
)

data class ChatState(
    val sessionId: String? = null,
    val progress: String = "",
    val lastReply: String = "",
    val inFlight: Boolean = false,
    val sessions: List<ChatSessionItem> = emptyList(),
    val turns: List<HistoryTurn> = emptyList(),
)

sealed class ChatIntent {
    data object NewSession : ChatIntent()

    data class SelectSession(val id: String) : ChatIntent()

    data class Send(val text: String) : ChatIntent()
}

internal fun sessionTitle(turns: List<HistoryTurn>): String {
    val line = turns
        .firstOrNull { it.role == "user" }
        ?.content
        ?.lineSequence()
        ?.firstOrNull()
        ?.trim()
        .orEmpty()
    return if (line.isEmpty()) "New chat" else line.take(40)
}

internal fun lastAssistant(turns: List<HistoryTurn>): String =
    turns.lastOrNull { it.role == "assistant" }?.content.orEmpty()

internal fun visibleTurns(turns: List<HistoryTurn>): List<HistoryTurn> =
    turns.filter { it.role == "user" || it.role == "assistant" }
