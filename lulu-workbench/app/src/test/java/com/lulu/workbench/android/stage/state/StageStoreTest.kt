package com.lulu.workbench.android.stage.state

import com.lulu.workbench.android.agent.tools.stage.StagedItem
import com.lulu.workbench.android.stage.commands.StageCommands
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class StageStoreTest {
    @Test
    fun deepLinkOpensFileAndListOpenUsesBackToList() {
        val items = mutableListOf(
            StagedItem("stg_1", "F1", "A", "sess_a", "chat", "body"),
        )
        val store = StageStore(
            StageCommands(
                listAll = { items.map { it.copy(body = "") } },
                getOne = { id -> items.firstOrNull { it.id == id } },
                save = { id, title, body ->
                    val index = items.indexOfFirst { it.id == id }
                    if (index < 0) {
                        null
                    } else {
                        val next = items[index].copy(title = title, body = body)
                        items[index] = next
                        next
                    }
                },
                remove = { id -> items.removeAll { it.id == id } },
            ),
            "sess_a",
            "stg_1",
        )
        assertEquals("stg_1", store.state.opened?.id)
        assertEquals(false, store.state.openedFromList)
        store.dispatch(StageIntent.CloseFile)
        assertNull(store.state.opened)
        store.dispatch(StageIntent.Open("stg_1"))
        assertEquals(true, store.state.openedFromList)
        store.dispatch(StageIntent.Save("B", "new"))
        assertEquals("B", store.state.opened?.title)
        assertEquals("new", store.state.opened?.body)
    }

    @Test
    fun deleteClearsOpenedAndDropsItem() {
        val items = mutableListOf(
            StagedItem("stg_1", "F1", "A", "sess_a", "chat", "body"),
            StagedItem("stg_2", "F2", "B", "sess_a", "chat", "other"),
        )
        val store = StageStore(
            StageCommands(
                listAll = { items.toList() },
                getOne = { id -> items.firstOrNull { it.id == id } },
                save = { _, _, _ -> null },
                remove = { id -> items.removeAll { it.id == id } },
            ),
            "sess_a",
            "stg_1",
        )
        store.dispatch(StageIntent.Delete)
        assertNull(store.state.opened)
        assertEquals(listOf("stg_2"), store.state.items.map { it.id })
    }
}
