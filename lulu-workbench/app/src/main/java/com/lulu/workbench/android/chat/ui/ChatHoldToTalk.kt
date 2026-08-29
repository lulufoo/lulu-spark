package com.lulu.workbench.android.chat.ui

import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.unit.dp
import com.lulu.workbench.android.chat.state.VoicePhase

internal const val VoiceCancelSlopDp = 56f

internal fun voiceCancelArmed(fingerY: Float, originY: Float, slopPx: Float): Boolean =
    fingerY < originY - slopPx

internal fun voiceHoldEnabled(
    inFlight: Boolean,
    phase: VoicePhase,
    asrConfigured: Boolean,
): Boolean = !inFlight && phase != VoicePhase.Recognizing && asrConfigured

internal const val AsrMissingToast = "Speech recognition is not configured"

internal fun shouldToastVoiceSwitch(asrConfigured: Boolean, toVoice: Boolean): Boolean =
    toVoice && !asrConfigured

@Composable
internal fun ChatHoldToTalk(
    phase: VoicePhase,
    enabled: Boolean,
    onPress: () -> Unit,
    onRelease: (cancel: Boolean) -> Unit,
    modifier: Modifier = Modifier,
) {
    val colors = MaterialTheme.colorScheme
    val slopPx = with(LocalDensity.current) { VoiceCancelSlopDp.dp.toPx() }
    var holding by remember { mutableStateOf(false) }
    var cancelArmed by remember { mutableStateOf(false) }
    val label = when {
        phase == VoicePhase.Recognizing -> "Recognizing…"
        holding && cancelArmed -> "Release to cancel"
        holding -> "Release to send"
        else -> "Hold to talk"
    }
    val container = when {
        !enabled -> colors.surfaceVariant.copy(alpha = 0.55f)
        holding && cancelArmed -> colors.error
        holding || phase == VoicePhase.Recording -> colors.primary
        else -> colors.surfaceVariant
    }
    val content = when {
        !enabled -> colors.onSurfaceVariant
        holding && cancelArmed -> colors.onError
        holding || phase == VoicePhase.Recording -> colors.onPrimary
        else -> colors.onSurface
    }
    Surface(
        modifier = modifier.fillMaxWidth(),
        shape = RoundedCornerShape(20.dp),
        color = container,
    ) {
        Box(
            modifier = Modifier
                .fillMaxWidth()
                .heightIn(min = 40.dp)
                .then(
                    if (enabled) {
                        Modifier.pointerInput(slopPx) {
                            awaitEachGesture {
                                val down = awaitFirstDown()
                                holding = true
                                cancelArmed = false
                                onPress()
                                val originY = down.position.y
                                while (true) {
                                    val event = awaitPointerEvent()
                                    val change = event.changes.firstOrNull { it.id == down.id }
                                    if (change == null) {
                                        val cancel = cancelArmed
                                        holding = false
                                        cancelArmed = false
                                        onRelease(cancel)
                                        break
                                    }
                                    cancelArmed = voiceCancelArmed(
                                        fingerY = change.position.y,
                                        originY = originY,
                                        slopPx = slopPx,
                                    )
                                    change.consume()
                                    if (!change.pressed) {
                                        val cancel = cancelArmed
                                        holding = false
                                        cancelArmed = false
                                        onRelease(cancel)
                                        break
                                    }
                                }
                            }
                        }
                    } else {
                        Modifier
                    },
                ),
            contentAlignment = Alignment.Center,
        ) {
            Text(
                text = label,
                modifier = Modifier.padding(horizontal = 14.dp, vertical = 10.dp),
                style = MaterialTheme.typography.bodyLarge,
                color = content,
            )
        }
    }
}
