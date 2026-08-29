package com.lulu.workbench.android.settings.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.exclude
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.ime
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.safeDrawing
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material3.Button
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ExposedDropdownMenuBox
import androidx.compose.material3.ExposedDropdownMenuDefaults
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalFocusManager
import androidx.compose.ui.platform.LocalSoftwareKeyboardController
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.text.input.VisualTransformation
import androidx.compose.ui.unit.dp
import com.lulu.workbench.android.bind.state.BindState
import com.lulu.workbench.android.bind.ui.BindScreen
import com.lulu.workbench.android.llm.LlmPreset
import com.lulu.workbench.android.settings.state.SettingsState

internal const val SAVED_API_KEY_MASK = "••••••••"

internal fun apiKeyForSave(draft: String): String {
    val trimmed = draft.trim()
    return if (trimmed.isEmpty() || trimmed == SAVED_API_KEY_MASK) "" else trimmed
}

@Composable
fun SettingsScreen(
    state: SettingsState,
    bindState: BindState,
    onBack: () -> Unit,
    onSelect: (String) -> Unit,
    onSave: (baseUrl: String, model: String, apiKey: String) -> Unit,
    onReset: () -> Unit,
    onSaveAsr: (appId: String, secretId: String, secretKey: String) -> Unit,
    onClearAsr: () -> Unit,
    onStartScan: () -> Unit,
    modifier: Modifier = Modifier,
) {
    var baseUrl by remember(state.active.id, state.active.baseUrl) {
        mutableStateOf(state.active.baseUrl)
    }
    var model by remember(state.active.id, state.active.model) {
        mutableStateOf(state.active.model)
    }
    var apiKey by remember(state.active.id, state.active.hasApiKey) {
        mutableStateOf(if (state.active.hasApiKey) SAVED_API_KEY_MASK else "")
    }
    var asrAppId by remember(state.asr.appId) { mutableStateOf(state.asr.appId) }
    var asrSecretId by remember(state.asr.secretId) { mutableStateOf(state.asr.secretId) }
    var asrSecretKey by remember(state.asr.hasSecretKey) {
        mutableStateOf(if (state.asr.hasSecretKey) SAVED_API_KEY_MASK else "")
    }
    val selected = state.catalog.firstOrNull { it.id == state.active.id }
        ?: state.catalog.firstOrNull()
    val colors = MaterialTheme.colorScheme
    val focusManager = LocalFocusManager.current
    val keyboard = LocalSoftwareKeyboardController.current
    fun dismissIme() {
        focusManager.clearFocus(force = true)
        keyboard?.hide()
    }
    Surface(
        modifier = modifier
            .fillMaxSize()
            .dismissImeOnBackgroundTap { dismissIme() },
        color = colors.background,
    ) {
        Column(
            modifier = Modifier
                .fillMaxSize()
                .windowInsetsPadding(WindowInsets.safeDrawing.exclude(WindowInsets.ime)),
        ) {
            SettingsHeader(onBack = onBack)
            HorizontalDivider(color = colors.outline.copy(alpha = 0.55f))
            Column(
                modifier = Modifier
                    .weight(1f)
                    .imePadding()
                    .verticalScroll(rememberScrollState())
                    .padding(horizontal = 16.dp, vertical = 16.dp),
            ) {
                SettingsCard {
                    Text("Language model", style = MaterialTheme.typography.titleMedium)
                    Text(
                        "Provider, endpoint, and credentials for chat.",
                        style = MaterialTheme.typography.bodySmall,
                        color = colors.onSurfaceVariant,
                    )
                    ProviderMenu(
                        catalog = state.catalog,
                        selected = selected,
                        onSelect = onSelect,
                    )
                    SettingsTextField(
                        value = baseUrl,
                        onValueChange = { baseUrl = it },
                        label = "Base URL",
                    )
                    SettingsTextField(
                        value = model,
                        onValueChange = { model = it },
                        label = "Model",
                    )
                    SettingsTextField(
                        value = apiKey,
                        onValueChange = { next ->
                            apiKey = if (apiKey == SAVED_API_KEY_MASK) {
                                next.filterNot { it == '•' }
                            } else {
                                next
                            }
                        },
                        label = "API key",
                        placeholder = "Not set",
                        supporting = if (state.active.hasApiKey) "Saved on this device" else null,
                        visualTransformation = secretTransform(apiKey),
                    )
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.spacedBy(8.dp),
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        Button(
                            onClick = { onSave(baseUrl, model, apiKeyForSave(apiKey)) },
                            colors = settingsButtonColors(),
                        ) {
                            Text("Save")
                        }
                        TextButton(onClick = onReset) {
                            Text("Reset defaults", color = colors.onSurfaceVariant)
                        }
                    }
                }
                Spacer(modifier = Modifier.height(12.dp))
                SettingsCard {
                    Text("Speech recognition", style = MaterialTheme.typography.titleMedium)
                    Text(
                        "Tencent Cloud one-sentence ASR. Keys stay on this device.",
                        style = MaterialTheme.typography.bodySmall,
                        color = colors.onSurfaceVariant,
                    )
                    SettingsTextField(
                        value = asrAppId,
                        onValueChange = { asrAppId = it },
                        label = "App ID",
                    )
                    SettingsTextField(
                        value = asrSecretId,
                        onValueChange = { asrSecretId = it },
                        label = "Secret ID",
                    )
                    SettingsTextField(
                        value = asrSecretKey,
                        onValueChange = { next ->
                            asrSecretKey = if (asrSecretKey == SAVED_API_KEY_MASK) {
                                next.filterNot { it == '•' }
                            } else {
                                next
                            }
                        },
                        label = "Secret key",
                        placeholder = "Not set",
                        supporting = if (state.asr.hasSecretKey) "Saved on this device" else null,
                        visualTransformation = secretTransform(asrSecretKey),
                    )
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.spacedBy(8.dp),
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        Button(
                            onClick = {
                                onSaveAsr(
                                    asrAppId,
                                    asrSecretId,
                                    apiKeyForSave(asrSecretKey),
                                )
                            },
                            colors = settingsButtonColors(),
                        ) {
                            Text("Save")
                        }
                        TextButton(onClick = onClearAsr) {
                            Text("Clear keys", color = colors.onSurfaceVariant)
                        }
                    }
                }
                Spacer(modifier = Modifier.height(12.dp))
                SettingsCard {
                    BindScreen(state = bindState, onScan = onStartScan)
                }
            }
        }
    }
}

@Composable
private fun SettingsHeader(onBack: () -> Unit) {
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .padding(end = 16.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        IconButton(onClick = onBack) {
            Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Back")
        }
        Column(modifier = Modifier.padding(start = 4.dp, top = 12.dp, bottom = 12.dp)) {
            Text("Settings", style = MaterialTheme.typography.titleLarge)
            Text(
                "Model, speech, and device",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun ProviderMenu(
    catalog: List<LlmPreset>,
    selected: LlmPreset?,
    onSelect: (String) -> Unit,
) {
    var open by remember { mutableStateOf(false) }
    ExposedDropdownMenuBox(expanded = open, onExpandedChange = { open = it }) {
        SettingsTextField(
            value = selected?.label.orEmpty(),
            onValueChange = {},
            modifier = Modifier.menuAnchor(),
            label = "Provider",
            readOnly = true,
            enabled = catalog.isNotEmpty(),
            trailingIcon = { ExposedDropdownMenuDefaults.TrailingIcon(expanded = open) },
        )
        ExposedDropdownMenu(expanded = open, onDismissRequest = { open = false }) {
            catalog.forEach { preset ->
                DropdownMenuItem(
                    text = { Text(preset.label) },
                    onClick = {
                        onSelect(preset.id)
                        open = false
                    },
                    contentPadding = ExposedDropdownMenuDefaults.ItemContentPadding,
                )
            }
        }
    }
}

private fun secretTransform(value: String): VisualTransformation =
    if (value.isEmpty()) VisualTransformation.None else PasswordVisualTransformation()
