package com.lulu.workbench.android.chat.ui

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class ChatDrawerSwipeTest {
    @Test
    fun flingRightOpens() {
        assertTrue(settleDrawerOpen(10f, 1000f, 800f, wasOpen = false))
    }

    @Test
    fun flingLeftClosesFromOpen() {
        assertFalse(settleDrawerOpen(900f, 1000f, -450f, wasOpen = true))
    }

    @Test
    fun shortLeftDragClosesWhenOpen() {
        assertFalse(settleDrawerOpen(700f, 1000f, 0f, wasOpen = true))
    }

    @Test
    fun restNearOpenStaysOpen() {
        assertTrue(settleDrawerOpen(800f, 1000f, 0f, wasOpen = true))
    }

    @Test
    fun restUsesPositionWhenClosed() {
        assertTrue(settleDrawerOpen(400f, 1000f, 0f, wasOpen = false))
        assertFalse(settleDrawerOpen(200f, 1000f, 0f, wasOpen = false))
    }

    @Test
    fun rubberSoftensPastEnds() {
        assertEquals(0f, rubberOffset(0f, 0f, 100f))
        assertTrue(rubberOffset(-40f, 0f, 100f) > -40f)
        assertTrue(rubberOffset(140f, 0f, 100f) < 140f)
    }
}
