package com.lulu.workbench.android.markdown.parse

import com.lulu.workbench.android.markdown.model.MdBlock
import com.lulu.workbench.android.markdown.model.MdInline
import org.commonmark.ext.gfm.strikethrough.Strikethrough
import org.commonmark.ext.gfm.strikethrough.StrikethroughExtension
import org.commonmark.ext.gfm.tables.TableBlock
import org.commonmark.ext.gfm.tables.TableBody
import org.commonmark.ext.gfm.tables.TableCell
import org.commonmark.ext.gfm.tables.TableHead
import org.commonmark.ext.gfm.tables.TableRow
import org.commonmark.ext.gfm.tables.TablesExtension
import org.commonmark.node.BlockQuote
import org.commonmark.node.BulletList
import org.commonmark.node.Code
import org.commonmark.node.Emphasis
import org.commonmark.node.FencedCodeBlock
import org.commonmark.node.HardLineBreak
import org.commonmark.node.Heading
import org.commonmark.node.Image
import org.commonmark.node.IndentedCodeBlock
import org.commonmark.node.Link
import org.commonmark.node.ListItem
import org.commonmark.node.Node
import org.commonmark.node.OrderedList
import org.commonmark.node.Paragraph
import org.commonmark.node.SoftLineBreak
import org.commonmark.node.StrongEmphasis
import org.commonmark.node.Text
import org.commonmark.node.ThematicBreak
import org.commonmark.parser.Parser

internal fun parseMdBlocks(source: String): List<MdBlock> {
    val root = PARSER.parse(source)
    val blocks = mutableListOf<MdBlock>()
    collectBlocks(root, blocks)
    return blocks
}

private val PARSER: Parser =
    Parser.builder()
        .extensions(
            listOf(
                TablesExtension.create(),
                StrikethroughExtension.create(),
            ),
        )
        .build()

private fun collectBlocks(node: Node, out: MutableList<MdBlock>) {
    var child = node.firstChild
    while (child != null) {
        when (child) {
            is Heading -> out.add(MdBlock.Heading(child.level, inlineSpans(child)))
            is Paragraph -> out.add(MdBlock.Paragraph(inlineSpans(child)))
            is BlockQuote -> out.add(MdBlock.Quote(quoteSpans(child)))
            is BulletList -> collectList(child, ordered = false, start = 1, out)
            is OrderedList -> collectList(child, ordered = true, start = child.markerStartNumber ?: 1, out)
            is FencedCodeBlock -> out.add(MdBlock.Code(child.info.orEmpty().trim(), child.literal.trimEnd('\n')))
            is IndentedCodeBlock -> out.add(MdBlock.Code("", child.literal.trimEnd('\n')))
            is ThematicBreak -> out.add(MdBlock.Rule)
            is TableBlock -> out.add(readTable(child))
        }
        child = child.next
    }
}

private fun collectList(list: Node, ordered: Boolean, start: Int, out: MutableList<MdBlock>) {
    var index = start
    var item = list.firstChild
    while (item != null) {
        if (item is ListItem) {
            out.add(MdBlock.ListItem(ordered, index, listItemSpans(item)))
            index += 1
        }
        item = item.next
    }
}

private fun quoteSpans(quote: BlockQuote): List<MdInline> {
    val spans = mutableListOf<MdInline>()
    var child = quote.firstChild
    while (child != null) {
        if (child is Paragraph) {
            if (spans.isNotEmpty()) spans.add(MdInline("\n"))
            spans.addAll(inlineSpans(child))
        }
        child = child.next
    }
    return spans
}

private fun listItemSpans(item: ListItem): List<MdInline> {
    val spans = mutableListOf<MdInline>()
    var child = item.firstChild
    while (child != null) {
        if (child is Paragraph) {
            if (spans.isNotEmpty()) spans.add(MdInline("\n"))
            spans.addAll(inlineSpans(child))
        }
        child = child.next
    }
    return spans
}

private fun readTable(table: TableBlock): MdBlock.Table {
    val headers = mutableListOf<List<MdInline>>()
    val rows = mutableListOf<List<List<MdInline>>>()
    var section = table.firstChild
    while (section != null) {
        when (section) {
            is TableHead -> {
                var row = section.firstChild
                while (row != null) {
                    if (row is TableRow && headers.isEmpty()) {
                        headers.addAll(rowCells(row))
                    }
                    row = row.next
                }
            }
            is TableBody -> {
                var row = section.firstChild
                while (row != null) {
                    if (row is TableRow) rows.add(rowCells(row))
                    row = row.next
                }
            }
        }
        section = section.next
    }
    return MdBlock.Table(headers, rows)
}

private fun rowCells(row: TableRow): List<List<MdInline>> {
    val cells = mutableListOf<List<MdInline>>()
    var cell = row.firstChild
    while (cell != null) {
        if (cell is TableCell) cells.add(inlineSpans(cell))
        cell = cell.next
    }
    return cells
}

private fun inlineSpans(node: Node): List<MdInline> {
    val out = mutableListOf<MdInline>()
    var child = node.firstChild
    while (child != null) {
        when (child) {
            is Text -> out.add(MdInline(child.literal))
            is Code -> out.add(MdInline(child.literal, code = true))
            is SoftLineBreak, is HardLineBreak -> out.add(MdInline("\n"))
            is Emphasis -> out.addAll(inlineSpans(child).map { it.copy(italic = true) })
            is StrongEmphasis -> out.addAll(inlineSpans(child).map { it.copy(bold = true) })
            is Strikethrough -> out.addAll(inlineSpans(child).map { it.copy(strike = true) })
            is Link -> {
                val url = child.destination
                out.addAll(inlineSpans(child).map { it.copy(link = url) })
            }
            is Image -> {
                val alt = inlineSpans(child).joinToString("") { it.text }
                if (alt.isNotEmpty()) out.add(MdInline(alt))
            }
        }
        child = child.next
    }
    return out
}
