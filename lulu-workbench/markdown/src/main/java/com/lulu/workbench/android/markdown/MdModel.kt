package com.lulu.workbench.android.markdown

internal sealed class MdBlock {
    data class Heading(val level: Int, val spans: List<MdInline>) : MdBlock()

    data class Paragraph(val spans: List<MdInline>) : MdBlock()

    data class Quote(val spans: List<MdInline>) : MdBlock()

    data class ListItem(val ordered: Boolean, val index: Int, val spans: List<MdInline>) : MdBlock()

    data class Code(val language: String, val text: String) : MdBlock()

    data class Table(
        val headers: List<List<MdInline>>,
        val rows: List<List<List<MdInline>>>,
    ) : MdBlock()

    data object Rule : MdBlock()
}

internal data class MdInline(
    val text: String,
    val bold: Boolean = false,
    val italic: Boolean = false,
    val code: Boolean = false,
    val strike: Boolean = false,
    val link: String? = null,
)
