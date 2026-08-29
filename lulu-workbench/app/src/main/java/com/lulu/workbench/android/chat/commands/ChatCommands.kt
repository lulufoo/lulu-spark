package com.lulu.workbench.android.chat.commands

import com.lulu.workbench.android.agent.facade.AgentFacade
import com.lulu.workbench.android.agent.loop.TurnProgress
import com.lulu.workbench.android.agent.session.HistoryTurn
import com.lulu.workbench.android.agent.session.SessionId

class ChatCommands(
    private val create: () -> SessionId,
    private val sendTurn: (SessionId, String, (TurnProgress) -> Unit) -> Unit,
    private val list: () -> List<SessionId> = { emptyList() },
    private val turnsOf: (SessionId) -> List<HistoryTurn> = { emptyList() },
) {
    constructor(agent: AgentFacade) : this(
        create = { agent.createSession() },
        sendTurn = { id, text, onProgress -> agent.loop(id).send(text, onProgress) },
        list = { agent.listSessions() },
        turnsOf = { agent.loadTurns(it) },
    )

    fun createSession(): SessionId = create()

    fun listSessions(): List<SessionId> = list()

    fun turns(sessionId: SessionId): List<HistoryTurn> = turnsOf(sessionId)

    fun send(sessionId: SessionId, text: String, onProgress: (TurnProgress) -> Unit) {
        sendTurn(sessionId, text, onProgress)
    }
}
