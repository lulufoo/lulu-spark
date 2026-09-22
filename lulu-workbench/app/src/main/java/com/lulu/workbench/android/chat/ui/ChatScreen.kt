package com.lulu.workbench.android.chat.ui

import androidx.activity.compose.BackHandler
import androidx.compose.animation.core.Animatable
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.width
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.shadow
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.dp
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.core.content.ContextCompat
import android.Manifest
import android.content.pm.PackageManager
import com.lulu.workbench.android.HomeEdgeAction
import com.lulu.workbench.android.chat.ChatViewModel
import com.lulu.workbench.android.chat.state.ChatIntent
import com.lulu.workbench.android.homeEdgeAction
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.LifecycleEventObserver
import androidx.lifecycle.compose.LocalLifecycleOwner
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import kotlin.math.roundToInt
import kotlinx.coroutines.Job
import kotlinx.coroutines.launch

@Composable
fun ChatScreen(
    viewModel: ChatViewModel,
    onOpenSettings: () -> Unit,
    onOpenStaged: (String) -> Unit,
    onOpenStagedAll: () -> Unit,
    modifier: Modifier = Modifier,
) {
    val state by viewModel.state.collectAsStateWithLifecycle()
    var drawerOpen by remember { mutableStateOf(false) }
    val context = LocalContext.current
    val micLauncher = rememberLauncherForActivityResult(
        ActivityResultContracts.RequestPermission(),
    ) { granted ->
        if (!granted) viewModel.dispatch(ChatIntent.MicDenied)
    }
    val lifecycleOwner = LocalLifecycleOwner.current
    DisposableEffect(lifecycleOwner) {
        val observer = LifecycleEventObserver { _, event ->
            if (event == Lifecycle.Event.ON_RESUME) {
                viewModel.dispatch(ChatIntent.RefreshAsr)
                viewModel.dispatch(ChatIntent.RefreshStaged)
            }
        }
        lifecycleOwner.lifecycle.addObserver(observer)
        onDispose { lifecycleOwner.lifecycle.removeObserver(observer) }
    }
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
                state = state,
                onNewSession = {
                    viewModel.dispatch(ChatIntent.NewSession)
                    settle(false)
                },
                onSelectSession = { id ->
                    if (id != state.sessionId) {
                        transcriptReady = false
                        viewModel.dispatch(ChatIntent.SelectSession(id))
                    }
                    settle(false)
                },
                onDeleteSession = { id ->
                    viewModel.dispatch(ChatIntent.DeleteSession(id))
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
                    state = state,
                    transcriptReady = transcriptReady,
                    onOpenDrawer = { settle(true) },
                    onNewSession = { viewModel.dispatch(ChatIntent.NewSession) },
                    onOpenStaged = onOpenStaged,
                    onOpenStagedAll = onOpenStagedAll,
                    onSend = { text -> viewModel.dispatch(ChatIntent.Send(text)) },
                    onVoicePress = {
                        val granted = ContextCompat.checkSelfPermission(
                            context,
                            Manifest.permission.RECORD_AUDIO,
                        ) == PackageManager.PERMISSION_GRANTED
                        if (granted) viewModel.dispatch(ChatIntent.VoicePress)
                        else micLauncher.launch(Manifest.permission.RECORD_AUDIO)
                    },
                    onVoiceRelease = { cancel ->
                        viewModel.dispatch(ChatIntent.VoiceRelease(cancel))
                    },
                    onVoiceHintShown = { viewModel.dispatch(ChatIntent.ClearVoiceHint) },
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
