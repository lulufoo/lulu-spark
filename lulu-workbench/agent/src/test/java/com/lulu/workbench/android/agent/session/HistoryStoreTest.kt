package com.lulu.workbench.android.agent.session

import com.lulu.workbench.android.storage.MemoryStorage
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class HistoryStoreTest {
    @Test
    fun createPersistsJsonTurnsOnHistoryPath() {
        val storage = MemoryStorage()
        val sessions = SessionRegistry(storage)
        val id = sessions.create()
        sessions.saveTurns(
            id,
            listOf(
                HistoryTurn("user", "hi"),
                HistoryTurn("assistant", "yo"),
            ),
        )
        val raw = storage.read(sessions.roots(id).historyPath)!!.decodeToString()
        assertTrue(raw.contains("\"role\":\"user\""))
        assertEquals("hi", sessions.loadTurns(id).first().content)
        assertTrue(storage.read("sessions/${id.value}/tools") == null)
    }
}
