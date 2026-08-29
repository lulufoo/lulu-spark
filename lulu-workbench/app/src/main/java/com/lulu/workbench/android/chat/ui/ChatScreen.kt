package com.lulu.workbench.android.chat.ui

import androidx.activity.compose.BackHandler
import androidx.compose.animation.core.Animatable
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.ime
import androidx.compose.foundation.layout.navigationBars
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.statusBars
import androidx.compose.foundation.layout.union
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.Menu
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalFocusManager
import androidx.compose.ui.platform.LocalSoftwareKeyboardController
import androidx.compose.ui.draw.shadow
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.dp
import com.lulu.workbench.android.HomeEdgeAction
import com.lulu.workbench.android.chat.state.ChatIntent
import com.lulu.workbench.android.chat.state.ChatState
import com.lulu.workbench.android.chat.state.ChatStore
import com.lulu.workbench.android.homeEdgeAction
import kotlin.math.roundToInt
import kotlinx.coroutines.Job
import kotlinx.coroutines.launch

@Composable
fun ChatScreen(
    store: ChatStore,
    onOpenSettings: () -> Unit,
    modifier: Modifier = Modifier,
) {
    var drawerOpen by remember { mutableStateOf(false) }
    val density = LocalDensity.current
    val scope = rememberCoroutineScope()
    var paneX by remember { mutableFloatStateOf(0f) }
    var settleJob by remember { mutableStateOf<Job?>(null) }
    var transcriptReady by remember { mutableStateOf(true) }
    BoxWithConstraints(modifier = modifier.fillMaxSize()) {
        val drawerWidth = maxWidth * 0.78f
        val drawerWidthPx = with(density) { drawerWidth.toPx() }
        val edgePx = with(density) { 56.dp.toPx() }
        LaunchedEffect(drawerWidthPx) {
            paneX = if (drawerOpen) drawerWidthPx else 0f
        }
        LaunchedEffect(paneX, drawerOpen) {
            if (!drawerOpen && paneX < 2f) transcriptReady = true
        }
        fun settle(open: Boolean, velocityX: Float = 0f) {
            drawerOpen = open
            val start = paneX
            settleJob?.cancel()
            settleJob = scope.launch {
                val anim = Animatable(start)
                anim.updateBounds(0f, drawerWidthPx)
                anim.animateTo(
                    targetValue = if (open) drawerWidthPx else 0f,
                    animationSpec = DrawerSpring,
                    initialVelocity = velocityX,
                ) {
                    paneX = value
                }
                paneX = anim.value
            }
        }
        BackHandler {
            when (homeEdgeAction(drawerOpen)) {
                HomeEdgeAction.OpenDrawer -> settle(true)
                HomeEdgeAction.CloseDrawer -> settle(false)
            }
        }
        Box(
            modifier = Modifier
                .fillMaxSize()
                .homeDrawerSwipe(
                    drawerWidthPx = drawerWidthPx,
                    edgePx = edgePx,
                    isOpen = { drawerOpen || paneX > 8f },
                    currentOffset = { paneX },
                    onDrag = { x ->
                        settleJob?.cancel()
                        paneX = x
                    },
                    onEnd = { x, velocityX ->
                        paneX = x
                        settle(
                            settleDrawerOpen(
                                offset = x,
                                width = drawerWidthPx,
                                velocityX = velocityX,
                                wasOpen = drawerOpen,
                            ),
                            velocityX,
                        )
                    },
                ),
        ) {
            ChatDrawer(
                state = store.state,
                onNewSession = {
                    store.dispatch(ChatIntent.NewSession)
                    settle(false)
                },
                onSelectSession = { id ->
                    if (id != store.state.sessionId) {
                        transcriptReady = false
                        store.dispatch(ChatIntent.SelectSession(id))
                    }
                    settle(false)
                },
                onDeleteSession = { id ->
                    store.dispatch(ChatIntent.DeleteSession(id))
                },
                onOpenSettings = onOpenSettings,
                modifier = Modifier
                    .width(drawerWidth)
                    .fillMaxHeight(),
            )
            val progress = if (drawerWidthPx == 0f) 0f else (paneX / drawerWidthPx).coerceIn(0f, 1f)
            Surface(
                modifier = Modifier
                    .fillMaxSize()
                    .offset { IntOffset(paneX.roundToInt(), 0) }
                    .shadow(elevation = (12f * progress).dp),
                color = MaterialTheme.colorScheme.background,
            ) {
                ChatPane(
                    state = store.state,
                    transcriptReady = transcriptReady,
                    onOpenDrawer = { settle(true) },
                    onNewSession = { store.dispatch(ChatIntent.NewSession) },
                    onSend = { text -> store.dispatch(ChatIntent.Send(text)) },
                )
                if (progress > 0.02f) {
                    Box(
                        modifier = Modifier
                            .fillMaxSize()
                            .background(Color.Black.copy(alpha = 0.34f * progress))
                            .clickable(
                                enabled = drawerOpen && progress > 0.85f,
                                interactionSource = remember { MutableInteractionSource() },
                                indication = null,
                            ) { settle(false) },
                    )
                }
            }
        }
    }
}

@Composable
private fun ChatPane(
    state: ChatState,
    transcriptReady: Boolean,
    onOpenDrawer: () -> Unit,
    onNewSession: () -> Unit,
    onSend: (String) -> Unit,
) {
    var draft by remember { mutableStateOf("") }
    val focusManager = LocalFocusManager.current
    val keyboard = LocalSoftwareKeyboardController.current
    fun dismissIme() {
        focusManager.clearFocus(force = true)
        keyboard?.hide()
    }
    val sendEnabled = !state.inFlight && draft.isNotBlank()
    val title = state.sessions.firstOrNull { it.id == state.sessionId }?.title ?: "Workbench"
    fun submit() {
        if (!sendEnabled) return
        val text = draft
        draft = ""
        onSend(text)
    }
    Column(modifier = Modifier.fillMaxSize()) {
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .windowInsetsPadding(WindowInsets.statusBars)
                .dismissImeOnTap { dismissIme() },
            verticalAlignment = Alignment.CenterVertically,
        ) {
            IconButton(onClick = onOpenDrawer) {
                Icon(Icons.Filled.Menu, contentDescription = "Chats")
            }
            Text(
                text = title,
                modifier = Modifier.weight(1f),
                style = MaterialTheme.typography.titleMedium,
                color = MaterialTheme.colorScheme.onSurface,
                maxLines = 1,
            )
            IconButton(onClick = onNewSession, enabled = !state.inFlight) {
                Icon(Icons.Filled.Add, contentDescription = "New chat")
            }
        }
        HorizontalDivider(color = MaterialTheme.colorScheme.outline.copy(alpha = 0.55f))
        ChatTranscript(
            state = state,
            ready = transcriptReady,
            modifier = Modifier
                .weight(1f)
                .fillMaxWidth()
                .dismissImeOnTap { dismissIme() },
        )
        ChatComposer(
            draft = draft,
            onDraftChange = { draft = it },
            enabled = !state.inFlight,
            sendEnabled = sendEnabled,
            onSend = { submit() },
            modifier = Modifier.windowInsetsPadding(
                WindowInsets.ime.union(WindowInsets.navigationBars),
            ),
        )
    }
}
