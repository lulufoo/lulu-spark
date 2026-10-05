package com.lulu.spark.android.markdown.render

import androidx.compose.foundation.background
import androidx.compose.foundation.horizontalScroll
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
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.clipToBounds
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontStyle
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import com.lulu.spark.android.markdown.model.MdBlock
import com.lulu.spark.android.markdown.model.MdInline

@Composable
internal fun MdBlockView(block: MdBlock) {
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
            MdInlineView(block.spans, style)
        }
        is MdBlock.Paragraph -> MdInlineView(block.spans, type.bodyLarge)
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
                MdInlineView(
                    block.spans,
                    type.bodyLarge.copy(fontStyle = FontStyle.Italic, color = colors.onSurfaceVariant),
                    modifier = Modifier
                        .weight(1f)
                        .padding(start = 12.dp),
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
                MdInlineView(block.spans, type.bodyLarge, modifier = Modifier.weight(1f))
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
                Box(modifier = Modifier.fillMaxWidth().clipToBounds()) {
                    Text(
                        block.text,
                        modifier = Modifier.horizontalScroll(rememberScrollState()),
                        style = type.bodyMedium.copy(fontFamily = FontFamily.Monospace),
                        softWrap = false,
                    )
                }
            }
        }
        is MdBlock.Table -> MdTableView(block)
        MdBlock.Rule -> HorizontalDivider(
            modifier = Modifier
                .fillMaxWidth()
                .padding(vertical = 4.dp),
        )
    }
}

@Composable
private fun MdTableView(table: MdBlock.Table) {
    val colors = MaterialTheme.colorScheme
    val type = MaterialTheme.typography
    val colCount = table.headers.size
    if (colCount == 0) return
    Row(modifier = Modifier.fillMaxWidth()) {
        repeat(colCount) { col ->
            Column(
                modifier = Modifier
                    .weight(1f)
                    .widthIn(min = 0.dp),
            ) {
                MdCell(
                    table.headers[col],
                    type.labelLarge.copy(fontWeight = FontWeight.SemiBold),
                    header = true,
                )
                HorizontalDivider(color = colors.outline.copy(alpha = 0.5f))
                table.rows.forEach { row ->
                    MdCell(row.getOrElse(col) { emptyList() }, type.bodyMedium, header = false)
                }
            }
        }
    }
}

@Composable
private fun MdCell(
    spans: List<MdInline>,
    style: TextStyle,
    header: Boolean,
) {
    val colors = MaterialTheme.colorScheme
    Box(
        modifier = Modifier
            .fillMaxWidth()
            .then(if (header) Modifier.background(colors.surfaceVariant.copy(alpha = 0.6f)) else Modifier)
            .padding(horizontal = 8.dp, vertical = 8.dp),
    ) {
        MdInlineView(spans, style)
    }
}
