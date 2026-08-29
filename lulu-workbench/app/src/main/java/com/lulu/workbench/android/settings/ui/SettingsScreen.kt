package com.lulu.workbench.android.settings.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.safeDrawing
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ExposedDropdownMenuBox
import androidx.compose.material3.ExposedDropdownMenuDefaults
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.OutlinedTextFieldDefaults
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
    val selected = state.catalog.firstOrNull { it.id == state.active.id }
        ?: state.catalog.firstOrNull()
    val colors = MaterialTheme.colorScheme
    Surface(modifier = modifier.fillMaxSize(), color = colors.background) {
        Column(
            modifier = Modifier
                .fillMaxSize()
                .windowInsetsPadding(WindowInsets.safeDrawing),
        ) {
            SettingsHeader(onBack = onBack)
            HorizontalDivider(color = colors.outline.copy(alpha = 0.55f))
            Column(
                modifier = Modifier
                    .weight(1f)
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
                    OutlinedTextField(
                        value = baseUrl,
                        onValueChange = { baseUrl = it },
                        modifier = Modifier.fillMaxWidth(),
                        singleLine = true,
                        label = { Text("Base URL") },
                        colors = settingsFieldColors(),
                    )
                    OutlinedTextField(
                        value = model,
                        onValueChange = { model = it },
                        modifier = Modifier.fillMaxWidth(),
                        singleLine = true,
                        label = { Text("Model") },
                        colors = settingsFieldColors(),
                    )
                    OutlinedTextField(
                        value = apiKey,
                        onValueChange = { next ->
                            apiKey = if (apiKey == SAVED_API_KEY_MASK) {
                                next.filterNot { it == '•' }
                            } else {
                                next
                            }
                        },
                        modifier = Modifier.fillMaxWidth(),
                        singleLine = true,
                        label = { Text("API key") },
                        placeholder = { Text("Not set") },
                        supportingText = {
                            Text(
                                if (state.active.hasApiKey) {
                                    "Saved on this device"
                                } else {
                                    "Required to call the model"
                                },
                            )
                        },
                        visualTransformation = if (apiKey.isEmpty()) {
                            VisualTransformation.None
                        } else {
                            PasswordVisualTransformation()
                        },
                        colors = settingsFieldColors(),
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
                    BindScreen(state = bindState, onScan = onStartScan)
                }
            }
        }
    }
}

@Composable
private fun SettingsCard(content: @Composable () -> Unit) {
    Surface(
        modifier = Modifier.fillMaxWidth(),
        shape = RoundedCornerShape(12.dp),
        color = MaterialTheme.colorScheme.surface,
    ) {
        Column(
            modifier = Modifier.padding(20.dp),
            verticalArrangement = Arrangement.spacedBy(12.dp),
        ) {
            content()
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
                "Model and device",
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
        OutlinedTextField(
            value = selected?.label.orEmpty(),
            onValueChange = {},
            modifier = Modifier.menuAnchor().fillMaxWidth(),
            readOnly = true,
            enabled = catalog.isNotEmpty(),
            singleLine = true,
            label = { Text("Provider") },
            colors = settingsFieldColors(),
            supportingText = {
                val platform = selected?.id.orEmpty()
                if (platform.isNotEmpty()) Text("Platform: $platform")
            },
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

@Composable
private fun settingsButtonColors() = ButtonDefaults.buttonColors(
    containerColor = MaterialTheme.colorScheme.surfaceVariant,
    contentColor = MaterialTheme.colorScheme.onSurface,
)

@Composable
private fun settingsFieldColors() = OutlinedTextFieldDefaults.colors(
    focusedBorderColor = MaterialTheme.colorScheme.outline,
    unfocusedBorderColor = MaterialTheme.colorScheme.outline.copy(alpha = 0.65f),
    focusedLabelColor = MaterialTheme.colorScheme.onSurfaceVariant,
    unfocusedLabelColor = MaterialTheme.colorScheme.onSurfaceVariant,
    focusedTextColor = MaterialTheme.colorScheme.onSurface,
    unfocusedTextColor = MaterialTheme.colorScheme.onSurface,
    cursorColor = MaterialTheme.colorScheme.onSurfaceVariant,
    focusedPlaceholderColor = MaterialTheme.colorScheme.onSurfaceVariant,
    unfocusedPlaceholderColor = MaterialTheme.colorScheme.onSurfaceVariant,
    focusedSupportingTextColor = MaterialTheme.colorScheme.onSurfaceVariant,
    unfocusedSupportingTextColor = MaterialTheme.colorScheme.onSurfaceVariant,
)
