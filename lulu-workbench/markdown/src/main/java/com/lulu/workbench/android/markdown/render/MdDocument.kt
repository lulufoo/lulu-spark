package com.lulu.workbench.android.markdown.render

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import com.lulu.workbench.android.markdown.cache.cachedMdBlocks

@Composable
internal fun MdDocument(
    source: String,
    modifier: Modifier = Modifier,
) {
    val blocks = remember(source) { cachedMdBlocks(source) }
    Column(
        modifier = modifier.fillMaxWidth(),
        verticalArrangement = Arrangement.spacedBy(10.dp),
    ) {
        blocks.forEach { block -> MdBlockView(block) }
    }
}
