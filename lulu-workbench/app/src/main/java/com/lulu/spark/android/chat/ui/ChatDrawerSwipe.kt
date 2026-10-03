package com.lulu.spark.android.chat.ui

import androidx.compose.animation.core.spring
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.ui.Modifier
import androidx.compose.ui.input.pointer.PointerEventPass
import androidx.compose.ui.input.pointer.PointerInputScope
import androidx.compose.ui.input.pointer.changedToUpIgnoreConsumed
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.input.pointer.util.VelocityTracker
import kotlin.math.abs

internal val DrawerSpring = spring<Float>(
    dampingRatio = 0.86f,
    stiffness = 220f,
    visibilityThreshold = 0.5f,
)

internal const val DrawerOpenFling = 750f
internal const val DrawerCloseFling = 420f
internal const val DrawerOpenAt = 0.38f
internal const val DrawerStayOpenAt = 0.72f
internal const val DrawerRubber = 0.18f

internal fun rubberOffset(raw: Float, min: Float, max: Float, resist: Float = DrawerRubber): Float =
    when {
        raw < min -> min + (raw - min) * resist
        raw > max -> max + (raw - max) * resist
        else -> raw
    }

internal fun settleDrawerOpen(
    offset: Float,
    width: Float,
    velocityX: Float,
    wasOpen: Boolean,
): Boolean =
    when {
        velocityX > DrawerOpenFling -> true
        velocityX < -DrawerCloseFling -> false
        wasOpen -> offset >= width * DrawerStayOpenAt
        else -> offset >= width * DrawerOpenAt
    }

internal suspend fun PointerInputScope.detectHomeDrawerDrag(
    drawerWidthPx: Float,
    edgePx: Float,
    isOpen: () -> Boolean,
    currentOffset: () -> Float,
    onDrag: (Float) -> Unit,
    onEnd: (offset: Float, velocityX: Float) -> Unit,
) {
    val slop = viewConfiguration.touchSlop
    awaitEachGesture {
        val down = awaitFirstDown(
            requireUnconsumed = false,
            pass = PointerEventPass.Initial,
        )
        val open = isOpen()
        val alreadyMoved = currentOffset() > 8f
        if (!open && !alreadyMoved && down.position.x > edgePx) return@awaitEachGesture
        val tracker = VelocityTracker()
        tracker.addPosition(down.uptimeMillis, down.position)
        var dragging = false
        var raw = currentOffset()
        var last = down.position
        while (true) {
            val event = awaitPointerEvent(PointerEventPass.Initial)
            val change = event.changes.firstOrNull { it.id == down.id } ?: break
            tracker.addPosition(change.uptimeMillis, change.position)
            if (change.changedToUpIgnoreConsumed()) {
                if (dragging) onEnd(rubberOffset(raw, 0f, drawerWidthPx), tracker.calculateVelocity().x)
                break
            }
            val travel = change.position - down.position
            if (!dragging) {
                if (abs(travel.x) < slop && abs(travel.y) < slop) continue
                val horizontal = abs(travel.x) > abs(travel.y) * if (open || alreadyMoved) 0.85f else 1f
                if (!horizontal) return@awaitEachGesture
                dragging = true
                raw = currentOffset() + travel.x
                last = change.position
                change.consume()
                onDrag(rubberOffset(raw, 0f, drawerWidthPx))
                continue
            }
            raw += change.position.x - last.x
            last = change.position
            change.consume()
            onDrag(rubberOffset(raw, 0f, drawerWidthPx))
        }
    }
}

internal fun Modifier.homeDrawerSwipe(
    drawerWidthPx: Float,
    edgePx: Float,
    isOpen: () -> Boolean,
    currentOffset: () -> Float,
    onDrag: (Float) -> Unit,
    onEnd: (offset: Float, velocityX: Float) -> Unit,
): Modifier = pointerInput(drawerWidthPx, edgePx) {
    detectHomeDrawerDrag(
        drawerWidthPx = drawerWidthPx,
        edgePx = edgePx,
        isOpen = isOpen,
        currentOffset = currentOffset,
        onDrag = onDrag,
        onEnd = onEnd,
    )
}
