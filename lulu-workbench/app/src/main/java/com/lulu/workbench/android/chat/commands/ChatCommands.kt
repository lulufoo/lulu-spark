package com.lulu.workbench.android.chat.commands

import com.lulu.workbench.android.agent.facade.AgentFacade
import com.lulu.workbench.android.agent.loop.TurnProgress
import com.lulu.workbench.android.agent.session.SessionId

class ChatCommands(
    private val create: () -> SessionId,
    private val sendTurn: (SessionId, String, (TurnProgress) -> Unit) -> Unit,
) {
    constructor(agent: AgentFacade) : this(
        create = { agent.createSession() },
        sendTurn = { id, text, onProgress -> agent.loop(id).send(text, onProgress) },
    )

    fun createSession(): SessionId = create()

    fun send(sessionId: SessionId, text: String, onProgress: (TurnProgress) -> Unit) {
        sendTurn(sessionId, text, onProgress)
    }
}
