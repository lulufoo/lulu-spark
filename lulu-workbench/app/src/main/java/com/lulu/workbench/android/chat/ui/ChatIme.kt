package com.lulu.workbench.android.chat.ui

import android.os.Build
import android.view.Window
import android.view.WindowManager
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.ime
import androidx.compose.foundation.layout.navigationBars
import androidx.compose.foundation.layout.union
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.input.pointer.PointerEventPass
import androidx.compose.ui.input.pointer.changedToUpIgnoreConsumed
import androidx.compose.ui.input.pointer.pointerInput

internal fun applyChatWindowIme(window: Window, sdk: Int = Build.VERSION.SDK_INT) {
    val mode = when (chatImeOwner(sdk)) {
        ChatImeOwner.Compose -> WindowManager.LayoutParams.SOFT_INPUT_ADJUST_NOTHING
        ChatImeOwner.Window -> WindowManager.LayoutParams.SOFT_INPUT_ADJUST_RESIZE
    }
    window.setSoftInputMode(mode)
}

@Composable
internal fun Modifier.chatComposerImePadding(sdk: Int = Build.VERSION.SDK_INT): Modifier {
    val insets = when (chatImeOwner(sdk)) {
        ChatImeOwner.Compose -> WindowInsets.ime.union(WindowInsets.navigationBars)
        ChatImeOwner.Window -> WindowInsets.navigationBars
    }
    return windowInsetsPadding(insets)
}

internal fun Modifier.dismissImeOnTap(onDismiss: () -> Unit): Modifier =
    pointerInput(onDismiss) {
        val slop = viewConfiguration.touchSlop * 2
        awaitEachGesture {
            val down = awaitFirstDown(
                requireUnconsumed = false,
                pass = PointerEventPass.Initial,
            )
            val start = down.position
            while (true) {
                val event = awaitPointerEvent(PointerEventPass.Initial)
                val change = event.changes.firstOrNull { it.id == down.id } ?: break
                if (change.changedToUpIgnoreConsumed()) {
                    if ((change.position - start).getDistance() <= slop) onDismiss()
                    break
                }
            }
        }
    }
