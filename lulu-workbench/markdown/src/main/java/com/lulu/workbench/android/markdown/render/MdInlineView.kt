package com.lulu.workbench.android.markdown.render

import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
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
import com.lulu.workbench.android.markdown.model.MdInline

@Composable
internal fun MdInlineView(
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
    Text(text = annotated, style = style, modifier = modifier.fillMaxWidth())
}
