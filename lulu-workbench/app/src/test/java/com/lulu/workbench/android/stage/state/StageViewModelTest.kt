package com.lulu.workbench.android.stage.state

import com.lulu.workbench.android.agent.tools.stage.StagedItem
import com.lulu.workbench.android.stage.StageViewModel
import com.lulu.workbench.android.stage.commands.StageCommands
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class StageViewModelTest {
    @Test
    fun deepLinkOpensFileAndListOpenUsesBackToList() {
        val items = mutableListOf(
            StagedItem("stg_1", "F1", "A", "sess_a", "chat", "body"),
        )
        val store = StageViewModel(
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
        assertEquals("stg_1", store.state.value.opened?.id)
        assertEquals(false, store.state.value.openedFromList)
        store.dispatch(StageIntent.CloseFile)
        assertNull(store.state.value.opened)
        store.dispatch(StageIntent.Open("stg_1"))
        assertEquals(true, store.state.value.openedFromList)
        store.dispatch(StageIntent.Save("B", "new"))
        assertEquals("B", store.state.value.opened?.title)
        assertEquals("new", store.state.value.opened?.body)
    }

    @Test
    fun deleteClearsOpenedAndDropsItem() {
        val items = mutableListOf(
            StagedItem("stg_1", "F1", "A", "sess_a", "chat", "body"),
            StagedItem("stg_2", "F2", "B", "sess_a", "chat", "other"),
        )
        val store = StageViewModel(
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
        assertNull(store.state.value.opened)
        assertEquals(listOf("stg_2"), store.state.value.items.map { it.id })
    }
}
