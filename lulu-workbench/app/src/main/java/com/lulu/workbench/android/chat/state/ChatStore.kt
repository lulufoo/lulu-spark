package com.lulu.workbench.android.chat.state

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import com.lulu.workbench.android.agent.loop.TurnProgress
import com.lulu.workbench.android.agent.session.HistoryTurn
import com.lulu.workbench.android.agent.session.SessionId
import com.lulu.workbench.android.chat.commands.ChatCommands

class ChatStore(
    private val commands: ChatCommands,
    private val runOffMain: (() -> Unit) -> Unit = { it() },
    private val runOnMain: (() -> Unit) -> Unit = { it() },
) {
    var state: ChatState by mutableStateOf(restore())
        private set

    fun dispatch(intent: ChatIntent) {
        when (intent) {
            ChatIntent.NewSession -> {
                if (state.inFlight) return
                val id = commands.createSession()
                state = ChatState(sessionId = id.value, sessions = listed())
            }
            is ChatIntent.SelectSession -> {
                if (state.inFlight) return
                val loaded = sessionTurns(intent.id)
                state = state.copy(
                    sessionId = intent.id,
                    lastReply = lastAssistant(loaded),
                    progress = "",
                    turns = loaded,
                )
            }
            is ChatIntent.Send -> {
                val text = intent.text.trim()
                if (text.isEmpty() || state.inFlight) return
                val id = state.sessionId ?: commands.createSession().value
                val turns = state.turns + HistoryTurn("user", text)
                state = state.copy(
                    sessionId = id,
                    inFlight = true,
                    lastReply = lastAssistant(turns),
                    progress = "",
                    turns = turns,
                )
                runOffMain {
                    try {
                        commands.send(SessionId(id), text) { progress ->
                            runOnMain { applyProgress(id, progress) }
                        }
                    } catch (error: Exception) {
                        runOnMain {
                            applyProgress(id, TurnProgress.Finished(error.message ?: "Send failed."))
                        }
                    }
                }
            }
        }
    }

    private fun applyProgress(sessionId: String, progress: TurnProgress) {
        if (state.sessionId != sessionId) return
        state = when (progress) {
            TurnProgress.CallingLlm,
            is TurnProgress.CallingTool,
            -> state.copy(progress = progressHint(progress), inFlight = true)
            is TurnProgress.Finished -> {
                val turns = state.turns + HistoryTurn("assistant", progress.reply)
                state.copy(
                    progress = "",
                    lastReply = progress.reply,
                    inFlight = false,
                    sessions = listed(),
                    turns = turns,
                )
            }
        }
    }

    private fun restore(): ChatState {
        val items = listed()
        val current = items.firstOrNull() ?: return ChatState()
        val loaded = sessionTurns(current.id)
        return ChatState(
            sessionId = current.id,
            lastReply = lastAssistant(loaded),
            sessions = items,
            turns = loaded,
        )
    }

    private fun sessionTurns(id: String): List<HistoryTurn> =
        visibleTurns(commands.turns(SessionId(id)))

    private fun listed(): List<ChatSessionItem> =
        commands.listSessions().asReversed().map { id ->
            ChatSessionItem(id = id.value, title = sessionTitle(commands.turns(id)))
        }
}
