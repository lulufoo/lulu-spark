package com.lulu.workbench.android.chat.ui

import org.junit.Assert.assertEquals
import org.junit.Test

class ChatImePolicyTest {
    @Test
    fun composeOwnsImeFromApi30() {
        assertEquals(ChatImeOwner.Window, chatImeOwner(29))
        assertEquals(ChatImeOwner.Compose, chatImeOwner(30))
        assertEquals(ChatImeOwner.Compose, chatImeOwner(36))
    }

    @Test
    fun oneOwnerAtEverySdk() {
        for (sdk in 24..36) {
            val owners = ChatImeOwner.entries.count { it == chatImeOwner(sdk) }
            assertEquals("sdk $sdk must pick exactly one owner", 1, owners)
        }
    }
}
