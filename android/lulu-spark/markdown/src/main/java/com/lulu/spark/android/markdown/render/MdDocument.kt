package com.lulu.spark.android.markdown.render

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clipToBounds
import androidx.compose.ui.layout.layout
import androidx.compose.ui.unit.dp
import com.lulu.spark.android.markdown.cache.cachedMdBlocks

@Composable
internal fun MdDocument(
    source: String,
    modifier: Modifier = Modifier,
) {
    val blocks = remember(source) { cachedMdBlocks(source) }
    Column(
        modifier = modifier
            .fillMaxWidth()
            .lockToIncomingWidth()
            .clipToBounds(),
        verticalArrangement = Arrangement.spacedBy(10.dp),
    ) {
        blocks.forEach { block -> MdBlockView(block) }
    }
}

private fun Modifier.lockToIncomingWidth(): Modifier = layout { measurable, constraints ->
    if (!constraints.hasBoundedWidth) {
        val placeable = measurable.measure(constraints)
        return@layout layout(placeable.width, placeable.height) {
            placeable.place(0, 0)
        }
    }
    val width = constraints.maxWidth
    val placeable = measurable.measure(
        constraints.copy(minWidth = width, maxWidth = width),
    )
    layout(width, placeable.height) {
        placeable.place(0, 0)
    }
}
