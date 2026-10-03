package com.lulu.spark.android.agent.facade

import com.lulu.spark.android.agent.loop.AgentLoop
import com.lulu.spark.android.agent.session.HistoryTurn
import com.lulu.spark.android.agent.session.SessionId
import com.lulu.spark.android.agent.session.SessionRegistry
import com.lulu.spark.android.agent.tools.ToolDispatcher
import com.lulu.spark.android.agent.tools.fs.FsTools
import com.lulu.spark.android.agent.tools.stage.StageStore
import com.lulu.spark.android.agent.tools.stage.StagedItem
import com.lulu.spark.android.agent.tools.web.WebSearchTools
import com.lulu.spark.android.asr.AsrClient
import com.lulu.spark.android.asr.AsrFactory
import com.lulu.spark.android.llm.LlmClient
import com.lulu.spark.android.llm.LlmFactory
import com.lulu.spark.android.storage.Storage
import com.lulu.spark.android.storage.StorageFactory
import com.lulu.spark.android.wmcp.WmcpClient
import com.lulu.spark.android.wmcp.WmcpFactory
import java.io.File

class AgentFacade(
    private val llm: LlmClient,
    private val wmcp: WmcpClient,
    storage: Storage,
    web: WebSearchTools = WebSearchTools(storage),
) {
    private val sessions = SessionRegistry(storage)
    private val stages = StageStore(storage)
    private val tools = ToolDispatcher(FsTools(), wmcp, storage, web)
    private val loops = mutableMapOf<SessionId, AgentLoop>()

    fun listSessions(): List<SessionId> = sessions.list()

    fun createSession(): SessionId = sessions.create()

    fun deleteSession(sessionId: SessionId) {
        loops.remove(sessionId)
        sessions.delete(sessionId)
    }

    fun loadTurns(sessionId: SessionId): List<HistoryTurn> = sessions.loadTurns(sessionId)

    fun listStaged(): List<StagedItem> = stages.list()

    fun listStaged(sessionId: SessionId): List<StagedItem> = stages.listForSession(sessionId.value)

    fun getStaged(id: String): StagedItem? = stages.get(id)

    fun updateStaged(id: String, title: String, body: String): StagedItem? =
        stages.update(id, title, body)

    fun deleteStaged(id: String): Boolean = stages.delete(id)

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
    val webSearch: WebSearchTools,
) {
    companion object {
        fun create(filesDir: File): WorkbenchRuntime = create(StorageFactory.create(filesDir))

        fun createForTest(): WorkbenchRuntime = create(StorageFactory.createMemory())

        private fun create(storage: Storage): WorkbenchRuntime {
            val llm = LlmFactory.create(storage)
            val wmcp = WmcpFactory.create(storage)
            val asr = AsrFactory.create(storage)
            val web = WebSearchTools(storage)
            return WorkbenchRuntime(AgentFacade(llm, wmcp, storage, web), llm, wmcp, asr, web)
        }
    }
}
