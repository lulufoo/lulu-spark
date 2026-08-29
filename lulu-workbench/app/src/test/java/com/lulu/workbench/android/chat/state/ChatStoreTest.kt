package com.lulu.workbench.android.chat.state

import com.lulu.workbench.android.agent.loop.TurnProgress
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
}
