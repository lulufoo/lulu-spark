package com.lulu.workbench.android.chat.commands

import com.lulu.workbench.android.agent.facade.AgentFacade
import com.lulu.workbench.android.agent.loop.TurnProgress
import com.lulu.workbench.android.agent.session.HistoryTurn
import com.lulu.workbench.android.agent.session.SessionId
import com.lulu.workbench.android.agent.tools.stage.StagedItem
import com.lulu.workbench.android.asr.AsrAudioFormat
import com.lulu.workbench.android.asr.AsrClient

class ChatCommands(
    private val create: () -> SessionId,
    private val sendTurn: (SessionId, String, (TurnProgress) -> Unit) -> Unit,
    private val list: () -> List<SessionId> = { emptyList() },
    private val turnsOf: (SessionId) -> List<HistoryTurn> = { emptyList() },
    private val remove: (SessionId) -> Unit = {},
    private val asrReady: () -> Boolean = { false },
    private val transcribe: (ByteArray) -> String = { "" },
    private val recorder: VoiceRecorder = IdleVoiceRecorder,
    private val stagedOf: (SessionId) -> List<StagedItem> = { emptyList() },
) {
    constructor(
        agent: AgentFacade,
        asr: AsrClient,
        recorder: VoiceRecorder,
    ) : this(
        create = { agent.createSession() },
        sendTurn = { id, text, onProgress -> agent.loop(id).send(text, onProgress) },
        list = { agent.listSessions() },
        turnsOf = { agent.loadTurns(it) },
        remove = { agent.deleteSession(it) },
        asrReady = { asr.isConfigured() },
        transcribe = { audio -> asr.recognize(audio, AsrAudioFormat.Wav).text },
        recorder = recorder,
        stagedOf = { id -> agent.listStaged(id) },
    )

    fun createSession(): SessionId = create()

    fun listSessions(): List<SessionId> = list()

    fun turns(sessionId: SessionId): List<HistoryTurn> = turnsOf(sessionId)

    fun deleteSession(sessionId: SessionId) = remove(sessionId)

    fun send(sessionId: SessionId, text: String, onProgress: (TurnProgress) -> Unit) {
        sendTurn(sessionId, text, onProgress)
    }

    fun isAsrConfigured(): Boolean = asrReady()

    fun startVoice() = recorder.start()

    fun stopVoice(): ByteArray = recorder.stop()

    fun transcribe(wav: ByteArray): String = transcribe.invoke(wav)

    fun listStaged(sessionId: SessionId): List<StagedItem> = stagedOf(sessionId)
}