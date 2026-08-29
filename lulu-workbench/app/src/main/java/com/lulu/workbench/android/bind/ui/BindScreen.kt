package com.lulu.workbench.android.bind.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.material3.Button
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import com.lulu.workbench.android.bind.state.BindState

@Composable
fun BindScreen(
    state: BindState,
    onScan: () -> Unit,
    modifier: Modifier = Modifier,
) {
    val status = when {
        state.completing -> "Binding…"
        state.bound -> "Bound"
        else -> "Not bound"
    }
    val statusColor = when {
        state.completing -> MaterialTheme.colorScheme.primary
        state.bound -> MaterialTheme.colorScheme.primary
        else -> MaterialTheme.colorScheme.onSurfaceVariant
    }
    Column(
        modifier = modifier,
        verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        Text("Device", style = MaterialTheme.typography.titleMedium)
        Text(
            "Pair this phone with Mac Workbench.",
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        Text(status, style = MaterialTheme.typography.titleSmall, color = statusColor)
        if (state.error.isNotEmpty()) {
            Text(state.error, color = MaterialTheme.colorScheme.error)
        }
        Button(
            onClick = onScan,
            enabled = !state.completing,
            modifier = Modifier.fillMaxWidth(),
        ) {
            Text("Scan QR code")
        }
    }
}
