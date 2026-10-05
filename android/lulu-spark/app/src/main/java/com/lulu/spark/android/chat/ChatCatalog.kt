package com.lulu.spark.android.chat

import com.lulu.spark.android.agent.session.HistoryTurn
import com.lulu.spark.android.agent.session.SessionId
import com.lulu.spark.android.agent.session.sessionTitle
import com.lulu.spark.android.asr.AsrException
import com.lulu.spark.android.asr.AsrNotConfiguredException
import com.lulu.spark.android.chat.commands.ChatCommands
import com.lulu.spark.android.chat.state.ChatSessionItem
import com.lulu.spark.android.chat.state.ChatStagedItem
import com.lulu.spark.android.chat.state.ChatState
import com.lulu.spark.android.chat.state.visibleTurns
import com.lulu.spark.android.markdown.prefetchMarkdown
import com.lulu.spark.android.wmcp.McpLinkState

internal fun restoreChatState(commands: ChatCommands, mcpLink: McpLinkState): ChatState {
    val items = listSessionItems(commands)
    val current = items.firstOrNull() ?: return ChatState(
        asrConfigured = commands.isAsrConfigured(),
        mcpLink = mcpLink,
    )
    return ChatState(
        sessionId = current.id,
        sessions = items,
        asrConfigured = commands.isAsrConfigured(),
        mcpLink = mcpLink,
        stagedThisChat = listStagedItems(commands, current.id),
    )
}

internal fun listSessionItems(commands: ChatCommands): List<ChatSessionItem> =
    commands.listSessions().asReversed().map { id ->
        ChatSessionItem(id = id.value, title = sessionTitle(commands.turns(id)))
    }

internal fun listStagedItems(
    commands: ChatCommands,
    sessionId: String?,
): List<ChatStagedItem> {
    if (sessionId == null) return emptyList()
    return commands.listStaged(SessionId(sessionId)).map {
        ChatStagedItem(it.id, it.handle, it.title)
    }
}

internal fun loadVisibleTurns(commands: ChatCommands, id: String): List<HistoryTurn> {
    val loaded = visibleTurns(commands.turns(SessionId(id)))
    loaded.forEach { turn ->
        if (turn.role == "assistant") prefetchMarkdown(turn.content)
    }
    return loaded
}

internal fun voiceErrorHint(error: Exception): String =
    when (error) {
        is AsrNotConfiguredException -> "Recognition failed"
        is AsrException -> error.message?.takeIf { it.isNotBlank() } ?: "Recognition failed"
        else -> "Recognition failed"
    }
