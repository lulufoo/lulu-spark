package com.lulu.workbench.android.settings.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material3.Button
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ExposedDropdownMenuBox
import androidx.compose.material3.ExposedDropdownMenuDefaults
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedCard
import androidx.compose.material3.OutlinedTextField
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
    Column(
        modifier = modifier
            .verticalScroll(rememberScrollState())
            .padding(horizontal = 16.dp, vertical = 8.dp),
    ) {
        SettingsHeader(onBack = onBack)
        Spacer(modifier = Modifier.height(16.dp))
        OutlinedCard(modifier = Modifier.fillMaxWidth()) {
            Column(
                modifier = Modifier.padding(20.dp),
                verticalArrangement = Arrangement.spacedBy(12.dp),
            ) {
                Text("Language model", style = MaterialTheme.typography.titleMedium)
                Text(
                    "Provider, endpoint, and credentials for chat.",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
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
                )
                OutlinedTextField(
                    value = model,
                    onValueChange = { model = it },
                    modifier = Modifier.fillMaxWidth(),
                    singleLine = true,
                    label = { Text("Model") },
                )
                OutlinedTextField(
                    value = apiKey,
                    onValueChange = { next ->
                        apiKey = if (apiKey == SAVED_API_KEY_MASK) next.filterNot { it == '•' } else next
                    },
                    modifier = Modifier.fillMaxWidth(),
                    singleLine = true,
                    label = { Text("API key") },
                    placeholder = { Text("Not set") },
                    supportingText = {
                        Text(if (state.active.hasApiKey) "Saved on this device" else "Required to call the model")
                    },
                    visualTransformation = if (apiKey.isEmpty()) {
                        VisualTransformation.None
                    } else {
                        PasswordVisualTransformation()
                    },
                )
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.spacedBy(8.dp),
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    Button(onClick = { onSave(baseUrl, model, apiKeyForSave(apiKey)) }) {
                        Text("Save")
                    }
                    TextButton(onClick = onReset) { Text("Reset defaults") }
                }
            }
        }
        Spacer(modifier = Modifier.height(12.dp))
        OutlinedCard(modifier = Modifier.fillMaxWidth()) {
            BindScreen(state = bindState, onScan = onStartScan)
        }
    }
}

@Composable
private fun SettingsHeader(onBack: () -> Unit) {
    Row(verticalAlignment = Alignment.CenterVertically) {
        IconButton(onClick = onBack) {
            Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Back")
        }
        Column(modifier = Modifier.padding(start = 4.dp)) {
            Text("Settings", style = MaterialTheme.typography.headlineSmall)
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
