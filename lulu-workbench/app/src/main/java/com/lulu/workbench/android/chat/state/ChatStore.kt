package com.lulu.workbench.android.chat.state

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import com.lulu.workbench.android.agent.loop.TurnProgress
import com.lulu.workbench.android.agent.session.HistoryTurn
import com.lulu.workbench.android.agent.session.SessionId
import com.lulu.workbench.android.asr.AsrException
import com.lulu.workbench.android.asr.AsrNotConfiguredException
import com.lulu.workbench.android.asr.isVoiceTooShort
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
                abandonVoice()
                val id = commands.createSession()
                state = ChatState(
                    sessionId = id.value,
                    sessions = listed(),
                    asrConfigured = asrReady(),
                )
            }
            is ChatIntent.SelectSession -> {
                if (state.inFlight) return
                abandonVoice()
                val id = intent.id
                state = state.copy(
                    sessionId = id,
                    lastReply = "",
                    progress = "",
                    turns = emptyList(),
                    voicePhase = VoicePhase.Idle,
                    voiceHint = "",
                )
                runOffMain {
                    val loaded = sessionTurns(id)
                    runOnMain {
                        if (state.sessionId != id || state.inFlight) return@runOnMain
                        state = state.copy(
                            lastReply = lastAssistant(loaded),
                            turns = loaded,
                        )
                    }
                }
            }
            is ChatIntent.DeleteSession -> {
                if (state.inFlight) return
                abandonVoice()
                commands.deleteSession(SessionId(intent.id))
                val remaining = listed()
                if (state.sessionId != intent.id) {
                    state = state.copy(sessions = remaining)
                    return
                }
                val next = remaining.firstOrNull()
                if (next == null) {
                    state = ChatState(asrConfigured = asrReady())
                    return
                }
                val loaded = sessionTurns(next.id)
                state = ChatState(
                    sessionId = next.id,
                    lastReply = lastAssistant(loaded),
                    sessions = remaining,
                    turns = loaded,
                    asrConfigured = asrReady(),
                )
            }
            ChatIntent.RefreshAsr -> {
                state = state.copy(asrConfigured = asrReady())
            }
            ChatIntent.ClearVoiceHint -> {
                if (state.voiceHint.isNotEmpty()) {
                    state = state.copy(voiceHint = "")
                }
            }
            ChatIntent.VoicePress -> onVoicePress()
            is ChatIntent.VoiceRelease -> onVoiceRelease(intent.cancel)
            ChatIntent.MicDenied -> {
                state = state.copy(voiceHint = "Microphone permission denied")
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

    private fun onVoicePress() {
        if (state.inFlight || state.voicePhase != VoicePhase.Idle) return
        if (!asrReady()) {
            state = state.copy(asrConfigured = false)
            return
        }
        try {
            commands.startVoice()
        } catch (error: SecurityException) {
            state = state.copy(voiceHint = "Microphone permission denied")
            return
        } catch (error: Exception) {
            state = state.copy(voiceHint = "Microphone failed")
            return
        }
        state = state.copy(voicePhase = VoicePhase.Recording, voiceHint = "")
    }

    private fun onVoiceRelease(cancel: Boolean) {
        if (state.voicePhase != VoicePhase.Recording) return
        val wav = runCatching { commands.stopVoice() }.getOrDefault(ByteArray(0))
        if (cancel) {
            state = state.copy(voicePhase = VoicePhase.Idle, voiceHint = "")
            return
        }
        if (isVoiceTooShort(wav)) {
            state = state.copy(voicePhase = VoicePhase.Idle, voiceHint = "Too short")
            return
        }
        state = state.copy(voicePhase = VoicePhase.Recognizing, voiceHint = "")
        runOffMain {
            try {
                val text = commands.transcribe(wav).trim()
                runOnMain { finishRecognize(text, null) }
            } catch (error: Exception) {
                runOnMain { finishRecognize("", voiceErrorHint(error)) }
            }
        }
    }

    private fun finishRecognize(text: String, errorHint: String?) {
        if (state.voicePhase != VoicePhase.Recognizing) return
        state = state.copy(voicePhase = VoicePhase.Idle, voiceHint = errorHint.orEmpty())
        when {
            errorHint != null -> Unit
            text.isEmpty() -> state = state.copy(voiceHint = "No speech detected")
            else -> dispatch(ChatIntent.Send(text))
        }
    }

    private fun abandonVoice() {
        if (state.voicePhase == VoicePhase.Idle) return
        if (state.voicePhase == VoicePhase.Recording) {
            runCatching { commands.stopVoice() }
        }
        state = state.copy(voicePhase = VoicePhase.Idle, voiceHint = "")
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
        val current = items.firstOrNull() ?: return ChatState(asrConfigured = asrReady())
        val loaded = sessionTurns(current.id)
        return ChatState(
            sessionId = current.id,
            lastReply = lastAssistant(loaded),
            sessions = items,
            turns = loaded,
            asrConfigured = asrReady(),
        )
    }

    private fun asrReady(): Boolean = commands.isAsrConfigured()

    private fun sessionTurns(id: String): List<HistoryTurn> =
        visibleTurns(commands.turns(SessionId(id)))

    private fun voiceErrorHint(error: Exception): String =
        when (error) {
            is AsrNotConfiguredException -> "Recognition failed"
            is AsrException -> error.message?.takeIf { it.isNotBlank() } ?: "Recognition failed"
            else -> "Recognition failed"
        }

    private fun listed(): List<ChatSessionItem> =
        commands.listSessions().asReversed().map { id ->
            ChatSessionItem(id = id.value, title = sessionTitle(commands.turns(id)))
        }
}
