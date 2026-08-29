package com.lulu.workbench.android.markdown

import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class MdParseTest {
    @Test
    fun splitsHeadingsListsCodeAndQuote() {
        val blocks = parseMdBlocks(
            """
            # Title
            Hello **world** and `code`.
            - one
            - two
            1. first
            > note
            ```kotlin
            val x = 1
            ```
            ---
            """.trimIndent(),
        )
        assertEquals(8, blocks.size)
        assertEquals(MdBlock.Heading(1, listOf(MdInline("Title"))), blocks[0])
        val hello = blocks[1] as MdBlock.Paragraph
        assertEquals("Hello ", hello.spans[0].text)
        assertTrue(hello.spans[1].bold)
        assertEquals("world", hello.spans[1].text)
        assertTrue(hello.spans.any { it.code && it.text == "code" })
        assertEquals(MdBlock.ListItem(false, 1, listOf(MdInline("one"))), blocks[2])
        assertEquals(MdBlock.ListItem(false, 2, listOf(MdInline("two"))), blocks[3])
        assertEquals(MdBlock.ListItem(true, 1, listOf(MdInline("first"))), blocks[4])
        assertEquals(MdBlock.Quote(listOf(MdInline("note"))), blocks[5])
        assertEquals(MdBlock.Code("kotlin", "val x = 1"), blocks[6])
        assertEquals(MdBlock.Rule, blocks[7])
    }

    @Test
    fun keepsUnclosedFenceAsCode() {
        val blocks = parseMdBlocks("```\npartial")
        assertEquals(listOf(MdBlock.Code("", "partial")), blocks)
    }

    @Test
    fun inlinesBoldCodeItalicLinkAndStrike() {
        val spans = (parseMdBlocks("See **bold**, `x`, *i*, ~~old~~ and [docs](https://example.com).")
            .single() as MdBlock.Paragraph).spans
        assertTrue(spans.any { it.bold && it.text == "bold" })
        assertTrue(spans.any { it.code && it.text == "x" })
        assertTrue(spans.any { it.italic && it.text == "i" })
        assertTrue(spans.any { it.strike && it.text == "old" })
        val link = spans.single { it.link != null }
        assertEquals("docs", link.text)
        assertEquals("https://example.com", link.link)
    }

    @Test
    fun unpairedMarkersStayLiteral() {
        val spans = (parseMdBlocks("star * and dash **").single() as MdBlock.Paragraph).spans
        assertEquals("star * and dash **", spans.joinToString("") { it.text })
        assertTrue(spans.none { it.bold || it.italic })
    }

    @Test
    fun readsGfmTableFromLatestChat() {
        val blocks = parseMdBlocks(
            """
            笔记内容概览：

            | 维度 | 豆包 | 通义千问 |
            |------|------|----------|
            | 公司 | 字节跳动 | 阿里巴巴 |
            | 核心优势 | C端用户体验、语音交互 | 开源生态、企业服务 |
            """.trimIndent(),
        )
        val table = blocks.filterIsInstance<MdBlock.Table>().single()
        assertEquals(listOf("维度", "豆包", "通义千问"), table.headers.map { it.joinToString("") { span -> span.text } })
        assertEquals("公司", table.rows[0][0].joinToString("") { it.text })
        assertEquals("阿里巴巴", table.rows[0][2].joinToString("") { it.text })
        assertEquals(2, table.rows.size)
    }
}
