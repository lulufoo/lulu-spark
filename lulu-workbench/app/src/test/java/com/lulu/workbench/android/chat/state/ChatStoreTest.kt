package com.lulu.workbench.android.chat.state

import com.lulu.workbench.android.agent.loop.TurnProgress
import com.lulu.workbench.android.agent.session.HistoryTurn
import com.lulu.workbench.android.agent.session.SessionId
import com.lulu.workbench.android.chat.commands.ChatCommands
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class ChatStoreTest {
    @Test
    fun sendShowsRequestingThenClearsOnFinish() {
        val seen = mutableListOf<String>()
        lateinit var store: ChatStore
        store = ChatStore(
            ChatCommands(
                create = { SessionId("sess_test") },
                sendTurn = { _, _, onProgress ->
                    onProgress(TurnProgress.CallingLlm)
                    seen.add(store.state.progress)
                    onProgress(TurnProgress.CallingTool("read"))
                    seen.add(store.state.progress)
                    onProgress(TurnProgress.Finished("ok"))
                },
            ),
        )
        store.dispatch(ChatIntent.Send("  hello  "))
        assertEquals(listOf("Requesting…", "Calling read…"), seen)
        assertEquals("ok", store.state.lastReply)
        assertEquals("", store.state.progress)
        assertFalse(store.state.inFlight)
        assertEquals(listOf("user", "assistant"), store.state.turns.map { it.role })
        assertEquals("hello", store.state.turns[0].content)
        assertEquals("ok", store.state.turns[1].content)
    }

    @Test
    fun blankOrInFlightSendIsIgnored() {
        var sends = 0
        val store = ChatStore(
            ChatCommands(
                create = { SessionId("sess_test") },
                sendTurn = { _, _, onProgress ->
                    sends += 1
                    onProgress(TurnProgress.CallingLlm)
                },
            ),
        )
        store.dispatch(ChatIntent.Send("   "))
        assertEquals(0, sends)
        store.dispatch(ChatIntent.Send("hi"))
        assertTrue(store.state.inFlight)
        store.dispatch(ChatIntent.Send("again"))
        assertEquals(1, sends)
    }

    @Test
    fun sendExceptionClearsInFlight() {
        val store = ChatStore(
            ChatCommands(
                create = { SessionId("sess_test") },
                sendTurn = { _, _, _ ->
                    throw IllegalArgumentException("mcp tools/list failed 406")
                },
            ),
        )
        store.dispatch(ChatIntent.Send("hi"))
        assertFalse(store.state.inFlight)
        assertEquals("mcp tools/list failed 406", store.state.lastReply)
    }

    @Test
    fun restoreNewestSessionAndSelectLoadsReply() {
        val turns = mapOf(
            "sess_old" to listOf(
                HistoryTurn("user", "old question"),
                HistoryTurn("assistant", "old answer"),
            ),
            "sess_new" to listOf(
                HistoryTurn("user", "new question"),
                HistoryTurn("assistant", "new answer"),
            ),
        )
        val store = ChatStore(
            ChatCommands(
                create = { SessionId("sess_fresh") },
                sendTurn = { _, _, _ -> },
                list = { listOf(SessionId("sess_old"), SessionId("sess_new")) },
                turnsOf = { id -> turns.getValue(id.value) },
            ),
        )
        assertEquals("sess_new", store.state.sessionId)
        assertEquals("new answer", store.state.lastReply)
        assertEquals(listOf("user", "assistant"), store.state.turns.map { it.role })
        assertEquals("new question", store.state.turns.first().content)
        assertEquals(listOf("sess_new", "sess_old"), store.state.sessions.map { it.id })
        assertEquals("new question", store.state.sessions.first().title)
        store.dispatch(ChatIntent.SelectSession("sess_old"))
        assertEquals("sess_old", store.state.sessionId)
        assertEquals("old answer", store.state.lastReply)
        assertEquals("old question", store.state.turns.first().content)
        assertEquals("old answer", store.state.turns.last().content)
    }

    @Test
    fun newSessionIsIgnoredWhileInFlight() {
        val store = ChatStore(
            ChatCommands(
                create = { SessionId("sess_fresh") },
                sendTurn = { _, _, onProgress -> onProgress(TurnProgress.CallingLlm) },
            ),
        )
        store.dispatch(ChatIntent.Send("hi"))
        store.dispatch(ChatIntent.NewSession)
        store.dispatch(ChatIntent.SelectSession("other"))
        assertEquals("sess_fresh", store.state.sessionId)
        assertTrue(store.state.inFlight)
    }

    @Test
    fun sessionTitleUsesFirstUserLine() {
        assertEquals("New chat", sessionTitle(emptyList()))
        assertEquals(
            "查待办",
            sessionTitle(listOf(HistoryTurn("user", "查待办\n第二行"))),
        )
    }

    @Test
    fun newSessionClearsReply() {
        val created = mutableListOf<String>()
        val store = ChatStore(
            ChatCommands(
                create = {
                    val id = SessionId("sess_${created.size}")
                    created.add(id.value)
                    id
                },
                sendTurn = { _, _, onProgress -> onProgress(TurnProgress.Finished("done")) },
                list = { created.map { SessionId(it) } },
                turnsOf = { emptyList() },
            ),
        )
        store.dispatch(ChatIntent.Send("hi"))
        assertEquals("done", store.state.lastReply)
        store.dispatch(ChatIntent.NewSession)
        assertEquals("sess_1", store.state.sessionId)
        assertEquals("", store.state.lastReply)
        assertTrue(store.state.turns.isEmpty())
        assertEquals("New chat", store.state.sessions.first { it.id == "sess_1" }.title)
    }

    @Test
    fun sendKeepsPriorTurnsAndAppendsUserImmediately() {
        val prior = listOf(
            HistoryTurn("user", "first"),
            HistoryTurn("assistant", "reply-1"),
        )
        lateinit var store: ChatStore
        store = ChatStore(
            ChatCommands(
                create = { SessionId("sess_test") },
                sendTurn = { _, _, onProgress ->
                    assertEquals(listOf("first", "reply-1", "second"), store.state.turns.map { it.content })
                    onProgress(TurnProgress.CallingLlm)
                    onProgress(TurnProgress.Finished("reply-2"))
                },
                list = { listOf(SessionId("sess_test")) },
                turnsOf = { prior },
            ),
        )
        assertEquals(prior, store.state.turns)
        store.dispatch(ChatIntent.Send("second"))
        assertEquals(listOf("first", "reply-1", "second", "reply-2"), store.state.turns.map { it.content })
    }

    @Test
    fun deleteCurrentSessionSwitchesToNewestRemaining() {
        val ids = mutableListOf("sess_a", "sess_b")
        val store = ChatStore(
            ChatCommands(
                create = { SessionId("unused") },
                sendTurn = { _, _, _ -> },
                list = { ids.map { SessionId(it) } },
                turnsOf = { id ->
                    listOf(HistoryTurn("user", id.value), HistoryTurn("assistant", "a-${id.value}"))
                },
                remove = { id -> ids.remove(id.value) },
            ),
        )
        assertEquals("sess_b", store.state.sessionId)
        store.dispatch(ChatIntent.DeleteSession("sess_b"))
        assertEquals("sess_a", store.state.sessionId)
        assertEquals(listOf("sess_a"), store.state.sessions.map { it.id })
        assertEquals("sess_a", store.state.turns.first().content)
    }

    @Test
    fun deleteLastSessionClearsChat() {
        val ids = mutableListOf("only")
        val store = ChatStore(
            ChatCommands(
                create = { SessionId("unused") },
                sendTurn = { _, _, _ -> },
                list = { ids.map { SessionId(it) } },
                turnsOf = { listOf(HistoryTurn("user", "hi")) },
                remove = { id -> ids.remove(id.value) },
            ),
        )
        store.dispatch(ChatIntent.DeleteSession("only"))
        assertEquals(null, store.state.sessionId)
        assertTrue(store.state.sessions.isEmpty())
        assertTrue(store.state.turns.isEmpty())
    }
}
