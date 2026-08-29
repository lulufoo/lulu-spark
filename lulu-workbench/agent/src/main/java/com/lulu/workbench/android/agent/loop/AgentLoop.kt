package com.lulu.workbench.android.agent.loop

import com.lulu.workbench.android.agent.session.HistoryTurn
import com.lulu.workbench.android.agent.session.SessionId
import com.lulu.workbench.android.agent.session.SessionRegistry
import com.lulu.workbench.android.agent.tools.ToolDispatcher
import com.lulu.workbench.android.llm.LlmClient
import com.lulu.workbench.android.llm.LlmException
import com.lulu.workbench.android.llm.LlmMessage
import com.lulu.workbench.android.log.LogModule
import com.lulu.workbench.android.log.WbLog
import java.util.UUID

sealed class TurnProgress {
    data object CallingLlm : TurnProgress()

    data class CallingTool(val name: String) : TurnProgress()

    data class Finished(val reply: String) : TurnProgress()
}

class AgentLoop(
    val sessionId: SessionId,
    private val llm: LlmClient,
    private val tools: ToolDispatcher,
    private val sessions: SessionRegistry,
) {
    @Volatile
    var inFlight: Boolean = false
        private set

    fun send(text: String, onProgress: (TurnProgress) -> Unit) {
        if (inFlight) {
            log.d("busy session=${sessionId.value}")
            return
        }
        inFlight = true
        val requestId = "req_" + UUID.randomUUID().toString().replace("-", "").take(12)
        log.i("send session=${sessionId.value} request=$requestId")
        try {
            runTurn(text, requestId, onProgress)
        } catch (error: Exception) {
            log.e("send failed session=${sessionId.value} request=$requestId")
            onProgress(TurnProgress.Finished(error.message ?: "send failed"))
        } finally {
            inFlight = false
        }
    }

    private fun runTurn(
        text: String,
        requestId: String,
        onProgress: (TurnProgress) -> Unit,
    ) {
        val turns = sessions.loadTurns(sessionId).toMutableList()
        turns.add(HistoryTurn(role = "user", content = text))
        val messages = turns.map { LlmMessage(it.role, it.content) }.toMutableList()
        val defs = tools.definitions()
        val toolRoot = sessions.roots(sessionId).toolPath
        var rounds = 0
        var callCount = 0
        while (true) {
            onProgress(TurnProgress.CallingLlm)
            val result = try {
                llm.complete(messages, defs)
            } catch (error: LlmException) {
                val reply = error.message ?: "llm failed"
                log.w("llm session=${sessionId.value} request=$requestId")
                turns.add(HistoryTurn(role = "assistant", content = reply))
                sessions.saveTurns(sessionId, turns)
                onProgress(TurnProgress.Finished(reply))
                return
            }
            log.i(
                "llm session=${sessionId.value} request=$requestId calls=${result.toolCalls.size}",
            )
            if (result.toolCalls.isEmpty()) {
                log.i("finished session=${sessionId.value} request=$requestId calls=0")
                turns.add(HistoryTurn(role = "assistant", content = result.text))
                sessions.saveTurns(sessionId, turns)
                onProgress(TurnProgress.Finished(result.text))
                return
            }
            if (rounds >= MAX_TOOL_ROUNDS || callCount + result.toolCalls.size > MAX_TOOL_CALLS) {
                val reply = "工具调用次数已达上限，未继续执行。"
                turns.add(HistoryTurn(role = "assistant", content = reply))
                sessions.saveTurns(sessionId, turns)
                onProgress(TurnProgress.Finished(reply))
                return
            }
            rounds += 1
            callCount += result.toolCalls.size
            messages.add(LlmMessage(role = "assistant", content = result.text, toolCalls = result.toolCalls))
            for (call in result.toolCalls) {
                log.i("tool session=${sessionId.value} request=$requestId name=${call.name}")
                onProgress(TurnProgress.CallingTool(call.name))
                val output = tools.call(call.name, call.arguments, toolRoot)
                messages.add(LlmMessage(role = "tool", content = output, toolCallId = call.id))
            }
        }
    }
}

private val log = WbLog.module(LogModule.AGENT)

private const val MAX_TOOL_ROUNDS = 8
private const val MAX_TOOL_CALLS = 16
