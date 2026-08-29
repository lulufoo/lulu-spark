package com.lulu.workbench.android.agent.facade

import com.lulu.workbench.android.agent.loop.AgentLoop
import com.lulu.workbench.android.agent.session.HistoryTurn
import com.lulu.workbench.android.agent.session.SessionId
import com.lulu.workbench.android.agent.session.SessionRegistry
import com.lulu.workbench.android.agent.tools.ToolDispatcher
import com.lulu.workbench.android.agent.tools.fs.FsTools
import com.lulu.workbench.android.asr.AsrClient
import com.lulu.workbench.android.asr.AsrFactory
import com.lulu.workbench.android.llm.LlmClient
import com.lulu.workbench.android.llm.LlmFactory
import com.lulu.workbench.android.storage.Storage
import com.lulu.workbench.android.storage.StorageFactory
import com.lulu.workbench.android.wmcp.WmcpClient
import com.lulu.workbench.android.wmcp.WmcpFactory
import java.io.File

class AgentFacade(
    private val llm: LlmClient,
    private val wmcp: WmcpClient,
    storage: Storage,
) {
    private val sessions = SessionRegistry(storage)
    private val tools = ToolDispatcher(FsTools(), wmcp, storage)
    private val loops = mutableMapOf<SessionId, AgentLoop>()

    fun listSessions(): List<SessionId> = sessions.list()

    fun createSession(): SessionId = sessions.create()

    fun deleteSession(sessionId: SessionId) {
        loops.remove(sessionId)
        sessions.delete(sessionId)
    }

    fun loadTurns(sessionId: SessionId): List<HistoryTurn> = sessions.loadTurns(sessionId)

    fun loop(sessionId: SessionId): AgentLoop =
        loops.getOrPut(sessionId) {
            AgentLoop(sessionId, llm, tools, sessions)
        }
}

class WorkbenchRuntime(
    val agent: AgentFacade,
    val llm: LlmClient,
    val wmcp: WmcpClient,
    val asr: AsrClient,
) {
    companion object {
        fun create(filesDir: File): WorkbenchRuntime = create(StorageFactory.create(filesDir))

        fun createForTest(): WorkbenchRuntime = create(StorageFactory.createMemory())

        private fun create(storage: Storage): WorkbenchRuntime {
            val llm = LlmFactory.create(storage)
            val wmcp = WmcpFactory.create(storage)
            val asr = AsrFactory.create(storage)
            return WorkbenchRuntime(AgentFacade(llm, wmcp, storage), llm, wmcp, asr)
        }
    }
}
