package com.lulu.workbench.android.markdown

import androidx.compose.foundation.background
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.IntrinsicSize
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.LinkAnnotation
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.TextLinkStyles
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontStyle
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextDecoration
import androidx.compose.ui.text.withLink
import androidx.compose.ui.text.withStyle
import androidx.compose.ui.unit.dp

@Composable
fun MarkdownBody(
    source: String,
    modifier: Modifier = Modifier,
) {
    val blocks = remember(source) { parseMdBlocks(source) }
    Column(
        modifier = modifier.fillMaxWidth(),
        verticalArrangement = Arrangement.spacedBy(10.dp),
    ) {
        blocks.forEach { block -> MdBlockView(block) }
    }
}

@Composable
private fun MdBlockView(block: MdBlock) {
    val colors = MaterialTheme.colorScheme
    val type = MaterialTheme.typography
    when (block) {
        is MdBlock.Heading -> {
            val style = when (block.level) {
                1 -> type.headlineSmall
                2 -> type.titleLarge
                3 -> type.titleMedium
                else -> type.titleSmall
            }
            InlineSpans(block.spans, style)
        }
        is MdBlock.Paragraph -> InlineSpans(block.spans, type.bodyLarge)
        is MdBlock.Quote -> {
            Row(
                modifier = Modifier
                    .fillMaxWidth()
                    .height(IntrinsicSize.Min),
            ) {
                Box(
                    modifier = Modifier
                        .width(3.dp)
                        .fillMaxHeight()
                        .clip(RoundedCornerShape(2.dp))
                        .background(colors.outline),
                )
                InlineSpans(
                    block.spans,
                    type.bodyLarge.copy(fontStyle = FontStyle.Italic, color = colors.onSurfaceVariant),
                    modifier = Modifier.padding(start = 12.dp),
                )
            }
        }
        is MdBlock.ListItem -> {
            val mark = if (block.ordered) "${block.index}. " else "•  "
            Row(modifier = Modifier.fillMaxWidth()) {
                Text(
                    mark,
                    modifier = Modifier.width(22.dp),
                    style = type.bodyLarge,
                    color = colors.onSurfaceVariant,
                )
                InlineSpans(block.spans, type.bodyLarge, modifier = Modifier.weight(1f))
            }
        }
        is MdBlock.Code -> {
            Column(
                modifier = Modifier
                    .fillMaxWidth()
                    .clip(RoundedCornerShape(10.dp))
                    .background(colors.surfaceVariant)
                    .padding(12.dp),
            ) {
                if (block.language.isNotEmpty()) {
                    Text(
                        block.language,
                        style = type.labelSmall,
                        color = colors.onSurfaceVariant,
                        modifier = Modifier.padding(bottom = 6.dp),
                    )
                }
                Text(
                    block.text,
                    style = type.bodyMedium.copy(fontFamily = FontFamily.Monospace),
                )
            }
        }
        is MdBlock.Table -> MdTableView(block)
        MdBlock.Rule -> HorizontalDivider(modifier = Modifier.padding(vertical = 4.dp))
    }
}

@Composable
private fun MdTableView(table: MdBlock.Table) {
    val colors = MaterialTheme.colorScheme
    val type = MaterialTheme.typography
    val colCount = table.headers.size
    if (colCount == 0) return
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .horizontalScroll(rememberScrollState()),
    ) {
        repeat(colCount) { col ->
            Column(modifier = Modifier.width(IntrinsicSize.Max)) {
                CellBox(table.headers[col], type.labelLarge.copy(fontWeight = FontWeight.SemiBold), header = true)
                HorizontalDivider(color = colors.outline.copy(alpha = 0.5f))
                table.rows.forEach { row ->
                    CellBox(row.getOrElse(col) { emptyList() }, type.bodyMedium, header = false)
                }
            }
        }
    }
}

@Composable
private fun CellBox(
    spans: List<MdInline>,
    style: TextStyle,
    header: Boolean,
) {
    val colors = MaterialTheme.colorScheme
    Box(
        modifier = Modifier
            .widthIn(min = 72.dp)
            .then(if (header) Modifier.background(colors.surfaceVariant.copy(alpha = 0.6f)) else Modifier)
            .padding(horizontal = 10.dp, vertical = 8.dp),
    ) {
        InlineSpans(spans, style)
    }
}

@Composable
private fun InlineSpans(
    spans: List<MdInline>,
    style: TextStyle,
    modifier: Modifier = Modifier,
) {
    val colors = MaterialTheme.colorScheme
    val annotated = remember(spans, colors) {
        val linkStyle = SpanStyle(color = colors.primary, textDecoration = TextDecoration.Underline)
        buildAnnotatedString {
            spans.forEach { span ->
                val styled = SpanStyle(
                    fontWeight = if (span.bold) FontWeight.Bold else FontWeight.Normal,
                    fontStyle = if (span.italic) FontStyle.Italic else FontStyle.Normal,
                    fontFamily = if (span.code) FontFamily.Monospace else FontFamily.Default,
                    background = if (span.code) colors.surfaceVariant else Color.Unspecified,
                    textDecoration = if (span.strike) TextDecoration.LineThrough else TextDecoration.None,
                )
                val url = span.link
                if (url != null) {
                    withLink(LinkAnnotation.Url(url, TextLinkStyles(style = linkStyle))) {
                        append(span.text)
                    }
                } else {
                    withStyle(styled) { append(span.text) }
                }
            }
        }
    }
    Text(text = annotated, style = style, modifier = modifier)
}
