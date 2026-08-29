package com.lulu.workbench.android.chat.state

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import com.lulu.workbench.android.agent.loop.TurnProgress
import com.lulu.workbench.android.agent.session.SessionId
import com.lulu.workbench.android.chat.commands.ChatCommands

class ChatStore(
    private val commands: ChatCommands,
    private val runOffMain: (() -> Unit) -> Unit = { it() },
    private val runOnMain: (() -> Unit) -> Unit = { it() },
) {
    var state: ChatState by mutableStateOf(ChatState())
        private set

    fun dispatch(intent: ChatIntent) {
        when (intent) {
            ChatIntent.NewSession -> {
                val id = commands.createSession()
                state = ChatState(sessionId = id.value)
            }
            is ChatIntent.Send -> {
                val text = intent.text.trim()
                if (text.isEmpty() || state.inFlight) return
                val id = state.sessionId ?: commands.createSession().value
                state = state.copy(sessionId = id, inFlight = true, lastReply = "", progress = "")
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
            is TurnProgress.Finished -> state.copy(
                progress = "",
                lastReply = progress.reply,
                inFlight = false,
            )
        }
    }
}
