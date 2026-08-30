package com.lulu.workbench.android.markdown

import org.junit.Assert.assertEquals
import org.junit.Assert.assertSame
import org.junit.Before
import org.junit.Test

class MdCacheTest {
    @Before
    fun clearCache() {
        MdBlockCache.clear()
    }

    @Test
    fun missParsesAndHitReturnsSameBlocks() {
        val source = "# Title\nHello **world**."
        val first = cachedMdBlocks(source)
        val second = cachedMdBlocks(source)
        assertEquals(1, MdBlockCache.size())
        assertSame(first, second)
        assertEquals(MdBlock.Heading(1, listOf(MdInline("Title"))), first[0])
    }

    @Test
    fun prefetchFillsCacheForLaterRead() {
        val source = "- one"
        prefetchMarkdown(source)
        assertEquals(1, MdBlockCache.size())
        assertEquals(
            MdBlock.ListItem(false, 1, listOf(MdInline("one"))),
            cachedMdBlocks(source).single(),
        )
    }

    @Test
    fun differentSourcesStaySeparate() {
        cachedMdBlocks("# A")
        cachedMdBlocks("# B")
        assertEquals(2, MdBlockCache.size())
        assertEquals("A", (cachedMdBlocks("# A").single() as MdBlock.Heading).spans.single().text)
        assertEquals("B", (cachedMdBlocks("# B").single() as MdBlock.Heading).spans.single().text)
    }
}
