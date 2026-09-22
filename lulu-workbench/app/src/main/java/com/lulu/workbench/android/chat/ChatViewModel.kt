package com.lulu.workbench.android.chat

import android.os.Handler
import android.os.Looper
import androidx.lifecycle.ViewModel
import com.lulu.workbench.android.agent.facade.WorkbenchRuntime
import com.lulu.workbench.android.agent.loop.TurnProgress
import com.lulu.workbench.android.agent.session.HistoryTurn
import com.lulu.workbench.android.agent.session.SessionId
import com.lulu.workbench.android.asr.isVoiceTooShort
import com.lulu.workbench.android.chat.commands.ChatCommands
import com.lulu.workbench.android.chat.state.ChatIntent
import com.lulu.workbench.android.chat.state.ChatState
import com.lulu.workbench.android.chat.state.VoicePhase
import com.lulu.workbench.android.chat.state.lastAssistant
import com.lulu.workbench.android.chat.state.progressHint
import com.lulu.workbench.android.markdown.prefetchMarkdown
import com.lulu.workbench.android.wmcp.IdleKeepAlive
import com.lulu.workbench.android.wmcp.McpKeepAlive
import dagger.hilt.android.lifecycle.HiltViewModel
import javax.inject.Inject
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update

@HiltViewModel
class ChatViewModel(
    private val commands: ChatCommands,
    private val keepAlive: McpKeepAlive = IdleKeepAlive,
    private val runOffMain: (() -> Unit) -> Unit = { it() },
    private val runOnMain: (() -> Unit) -> Unit = { it() },
    startKeepAlive: Boolean = false,
) : ViewModel() {
    @Inject
    constructor(
        commands: ChatCommands,
        runtime: WorkbenchRuntime,
    ) : this(
        commands,
        keepAlive = runtime.wmcp.keepAlive(),
        runOffMain = { block -> Thread { block() }.start() },
        runOnMain = { block -> Handler(Looper.getMainLooper()).post(block) },
        startKeepAlive = true,
    )

    private val _state = MutableStateFlow(restoreChatState(commands, keepAlive.state()))
    val state: StateFlow<ChatState> = _state.asStateFlow()

    init {
        if (startKeepAlive) keepAlive.start()
        keepAlive.addListener { next ->
            runOnMain {
                if (_state.value.mcpLink != next) {
                    _state.update { it.copy(mcpLink = next) }
                }
            }
        }
        val id = _state.value.sessionId
        if (id != null) {
            runOffMain {
                val loaded = loadVisibleTurns(commands, id)
                runOnMain {
                    if (_state.value.sessionId != id) return@runOnMain
                    _state.update {
                        it.copy(
                            lastReply = lastAssistant(loaded),
                            turns = loaded,
                        )
                    }
                }
            }
        }
    }

    fun dispatch(intent: ChatIntent) {
        when (intent) {
            ChatIntent.NewSession -> startNewSession()
            is ChatIntent.SelectSession -> selectSession(intent.id)
            is ChatIntent.DeleteSession -> deleteSession(intent.id)
            ChatIntent.RefreshAsr -> {
                _state.update { it.copy(asrConfigured = commands.isAsrConfigured()) }
            }
            ChatIntent.RefreshStaged -> {
                _state.update { it.copy(stagedThisChat = listStagedItems(commands, it.sessionId)) }
            }
            ChatIntent.ClearVoiceHint -> {
                if (_state.value.voiceHint.isNotEmpty()) {
                    _state.update { it.copy(voiceHint = "") }
                }
            }
            ChatIntent.VoicePress -> startVoice()
            is ChatIntent.VoiceRelease -> finishVoice(intent.cancel)
            ChatIntent.MicDenied -> {
                _state.update { it.copy(voiceHint = "Microphone permission denied") }
            }
            is ChatIntent.Send -> sendMessage(intent.text)
        }
    }

    private fun startNewSession() {
        if (_state.value.inFlight) return
        abandonVoice()
        val id = commands.createSession()
        _state.value = ChatState(
            sessionId = id.value,
            sessions = listSessionItems(commands),
            asrConfigured = commands.isAsrConfigured(),
            mcpLink = _state.value.mcpLink,
            stagedThisChat = listStagedItems(commands, id.value),
        )
    }

    private fun selectSession(id: String) {
        if (_state.value.inFlight) return
        abandonVoice()
        _state.update {
            it.copy(
                sessionId = id,
                lastReply = "",
                progress = "",
                turns = emptyList(),
                voicePhase = VoicePhase.Idle,
                voiceHint = "",
                stagedThisChat = listStagedItems(commands, id),
            )
        }
        loadSessionTurns(id)
    }

    private fun deleteSession(id: String) {
        if (_state.value.inFlight) return
        abandonVoice()
        commands.deleteSession(SessionId(id))
        val remaining = listSessionItems(commands)
        if (_state.value.sessionId != id) {
            _state.update { it.copy(sessions = remaining) }
            return
        }
        val next = remaining.firstOrNull()
        if (next == null) {
            _state.value = ChatState(
                asrConfigured = commands.isAsrConfigured(),
                mcpLink = _state.value.mcpLink,
            )
            return
        }
        _state.value = ChatState(
            sessionId = next.id,
            sessions = remaining,
            asrConfigured = commands.isAsrConfigured(),
            mcpLink = _state.value.mcpLink,
            stagedThisChat = listStagedItems(commands, next.id),
        )
        loadSessionTurns(next.id)
    }

    private fun sendMessage(text: String) {
        val trimmed = text.trim()
        if (trimmed.isEmpty() || _state.value.inFlight) return
        val id = _state.value.sessionId ?: commands.createSession().value
        val turns = _state.value.turns + HistoryTurn("user", trimmed)
        _state.update {
            it.copy(
                sessionId = id,
                inFlight = true,
                lastReply = lastAssistant(turns),
                progress = "",
                turns = turns,
            )
        }
        runOffMain {
            try {
                commands.send(SessionId(id), trimmed) { progress ->
                    if (progress is TurnProgress.Finished) {
                        prefetchMarkdown(progress.reply)
                    }
                    runOnMain { applyProgress(id, progress) }
                }
            } catch (error: Exception) {
                val reply = error.message ?: "Send failed."
                prefetchMarkdown(reply)
                runOnMain { applyProgress(id, TurnProgress.Finished(reply)) }
            }
        }
    }

    private fun startVoice() {
        if (_state.value.inFlight || _state.value.voicePhase != VoicePhase.Idle) return
        if (!commands.isAsrConfigured()) {
            _state.update { it.copy(asrConfigured = false) }
            return
        }
        try {
            commands.startVoice()
        } catch (error: SecurityException) {
            _state.update { it.copy(voiceHint = "Microphone permission denied") }
            return
        } catch (error: Exception) {
            _state.update { it.copy(voiceHint = "Microphone failed") }
            return
        }
        _state.update { it.copy(voicePhase = VoicePhase.Recording, voiceHint = "") }
    }

    private fun finishVoice(cancel: Boolean) {
        if (_state.value.voicePhase != VoicePhase.Recording) return
        val wav = runCatching { commands.stopVoice() }.getOrDefault(ByteArray(0))
        if (cancel) {
            _state.update { it.copy(voicePhase = VoicePhase.Idle, voiceHint = "") }
            return
        }
        if (isVoiceTooShort(wav)) {
            _state.update { it.copy(voicePhase = VoicePhase.Idle, voiceHint = "Too short") }
            return
        }
        _state.update { it.copy(voicePhase = VoicePhase.Recognizing, voiceHint = "") }
        runOffMain {
            try {
                val text = commands.transcribe(wav).trim()
                runOnMain { applyRecognize(text, null) }
            } catch (error: Exception) {
                runOnMain { applyRecognize("", voiceErrorHint(error)) }
            }
        }
    }

    private fun applyRecognize(text: String, errorHint: String?) {
        if (_state.value.voicePhase != VoicePhase.Recognizing) return
        _state.update { it.copy(voicePhase = VoicePhase.Idle, voiceHint = errorHint.orEmpty()) }
        when {
            errorHint != null -> Unit
            text.isEmpty() -> _state.update { it.copy(voiceHint = "No speech detected") }
            else -> dispatch(ChatIntent.Send(text))
        }
    }

    private fun abandonVoice() {
        if (_state.value.voicePhase == VoicePhase.Idle) return
        if (_state.value.voicePhase == VoicePhase.Recording) {
            runCatching { commands.stopVoice() }
        }
        _state.update { it.copy(voicePhase = VoicePhase.Idle, voiceHint = "") }
    }

    private fun loadSessionTurns(id: String) {
        runOffMain {
            val loaded = loadVisibleTurns(commands, id)
            runOnMain {
                if (_state.value.sessionId != id || _state.value.inFlight) return@runOnMain
                _state.update {
                    it.copy(
                        lastReply = lastAssistant(loaded),
                        turns = loaded,
                    )
                }
            }
        }
    }

    private fun applyProgress(sessionId: String, progress: TurnProgress) {
        if (_state.value.sessionId != sessionId) return
        _state.update { current ->
            when (progress) {
                TurnProgress.CallingLlm,
                is TurnProgress.CallingTool,
                -> current.copy(progress = progressHint(progress), inFlight = true)
                is TurnProgress.Finished -> {
                    val turns = current.turns + HistoryTurn("assistant", progress.reply)
                    current.copy(
                        progress = "",
                        lastReply = progress.reply,
                        inFlight = false,
                        sessions = listSessionItems(commands),
                        turns = turns,
                        stagedThisChat = listStagedItems(commands, sessionId),
                    )
                }
            }
        }
    }
}
