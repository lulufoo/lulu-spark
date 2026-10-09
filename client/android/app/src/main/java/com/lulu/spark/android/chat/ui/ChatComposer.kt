package com.lulu.spark.android.chat.ui

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.Send
import androidx.compose.material3.FilledIconButton
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.IconButtonDefaults
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalFocusManager
import androidx.compose.ui.platform.LocalSoftwareKeyboardController
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.unit.dp
import android.widget.Toast
import com.lulu.spark.android.chat.state.VoicePhase

@Composable
internal fun ChatComposer(
    draft: String,
    onDraftChange: (String) -> Unit,
    sendEnabled: Boolean,
    onSend: () -> Unit,
    voicePhase: VoicePhase,
    voiceHint: String,
    voiceEnabled: Boolean,
    asrConfigured: Boolean,
    onVoicePress: () -> Unit,
    onVoiceRelease: (cancel: Boolean) -> Unit,
    onVoiceHintShown: () -> Unit,
    modifier: Modifier = Modifier,
) {
    val colors = MaterialTheme.colorScheme
    val onSendLatest = rememberUpdatedState(onSend)
    val keyboardOptions = remember { KeyboardOptions(imeAction = ImeAction.Send) }
    val sendActions = remember {
        KeyboardActions(onSend = { onSendLatest.value() })
    }
    var voiceMode by remember { mutableStateOf(false) }
    val context = LocalContext.current
    LaunchedEffect(voiceHint) {
        if (voiceHint.isEmpty()) return@LaunchedEffect
        Toast.makeText(context, voiceHint, Toast.LENGTH_SHORT).show()
        onVoiceHintShown()
    }
    val focusManager = LocalFocusManager.current
    val keyboard = LocalSoftwareKeyboardController.current
    fun switchMode() {
        if (voicePhase == VoicePhase.Recognizing) return
        if (voicePhase == VoicePhase.Recording) onVoiceRelease(true)
        val toVoice = !voiceMode
        voiceMode = toVoice
        if (toVoice) {
            focusManager.clearFocus(force = true)
            keyboard?.hide()
            if (shouldToastVoiceSwitch(asrConfigured, toVoice = true)) {
                Toast.makeText(context, AsrMissingToast, Toast.LENGTH_SHORT).show()
            }
        }
    }
    Column(modifier = modifier.fillMaxWidth()) {
        HorizontalDivider(color = colors.outline.copy(alpha = 0.55f))
        Surface(color = colors.surface) {
            Row(
                modifier = Modifier.padding(start = 4.dp, end = 8.dp, top = 8.dp, bottom = 8.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                IconButton(
                    onClick = { switchMode() },
                    enabled = voicePhase != VoicePhase.Recognizing,
                ) {
                    if (voiceMode) {
                        Icon(ComposerKeyboard, contentDescription = "Keyboard")
                    } else {
                        Icon(ComposerMic, contentDescription = "Voice input")
                    }
                }
                if (voiceMode) {
                    ChatHoldToTalk(
                        phase = voicePhase,
                        enabled = voiceEnabled,
                        onPress = onVoicePress,
                        onRelease = onVoiceRelease,
                        modifier = Modifier.weight(1f),
                    )
                } else {
                    Surface(
                        modifier = Modifier.weight(1f),
                        shape = RoundedCornerShape(20.dp),
                        color = colors.surfaceVariant,
                    ) {
                        BasicTextField(
                            value = draft,
                            onValueChange = onDraftChange,
                            modifier = Modifier
                                .fillMaxWidth()
                                .padding(horizontal = 14.dp, vertical = 10.dp),
                            singleLine = true,
                            textStyle = MaterialTheme.typography.bodyLarge.copy(
                                color = colors.onSurface,
                            ),
                            cursorBrush = SolidColor(colors.onSurfaceVariant),
                            keyboardOptions = keyboardOptions,
                            keyboardActions = sendActions,
                            decorationBox = { inner ->
                                if (draft.isEmpty()) {
                                    Text(
                                        "Ask Lulu Spark…",
                                        style = MaterialTheme.typography.bodyLarge,
                                        color = colors.onSurfaceVariant,
                                    )
                                }
                                inner()
                            },
                        )
                    }
                    FilledIconButton(
                        onClick = onSend,
                        enabled = sendEnabled,
                        shape = CircleShape,
                        modifier = Modifier.padding(start = 8.dp),
                        colors = IconButtonDefaults.filledIconButtonColors(
                            containerColor = colors.primary,
                            contentColor = colors.onPrimary,
                            disabledContainerColor = colors.surfaceVariant,
                            disabledContentColor = colors.onSurfaceVariant,
                        ),
                    ) {
                        Icon(Icons.AutoMirrored.Filled.Send, contentDescription = "Send")
                    }
                }
            }
        }
    }
}
