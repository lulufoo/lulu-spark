package com.lulu.spark.android.markdown

import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import com.lulu.spark.android.markdown.render.MdDocument

/** Chat transcript surface: read-only document paint. Does not edit. */
@Composable
fun MarkdownBody(
    source: String,
    modifier: Modifier = Modifier,
) {
    MdDocument(source, modifier)
}
