package com.lulu.spark.android.settings.ui

import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.ui.Modifier
import androidx.compose.ui.input.pointer.changedToUpIgnoreConsumed
import androidx.compose.ui.input.pointer.pointerInput

internal fun Modifier.dismissImeOnBackgroundTap(onDismiss: () -> Unit): Modifier =
    pointerInput(onDismiss) {
        val slop = viewConfiguration.touchSlop * 2
        awaitEachGesture {
            val down = awaitFirstDown(requireUnconsumed = true)
            val start = down.position
            while (true) {
                val event = awaitPointerEvent()
                val change = event.changes.firstOrNull { it.id == down.id } ?: break
                val dragged = (change.position - start).getDistance() > slop
                if (dragged) break
                if (change.changedToUpIgnoreConsumed()) {
                    onDismiss()
                    break
                }
            }
        }
    }
