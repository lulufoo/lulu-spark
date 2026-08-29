package com.lulu.workbench.android.bind.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
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
    val colors = MaterialTheme.colorScheme
    val statusColor = when {
        state.completing -> colors.onSurface
        state.bound -> colors.onSurface
        else -> colors.onSurfaceVariant
    }
    Column(
        modifier = modifier,
        verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        Text("Device", style = MaterialTheme.typography.titleMedium)
        Text(
            "Pair this phone with Mac Workbench.",
            style = MaterialTheme.typography.bodySmall,
            color = colors.onSurfaceVariant,
        )
        Text(status, style = MaterialTheme.typography.titleSmall, color = statusColor)
        if (state.error.isNotEmpty()) {
            Text(state.error, color = colors.error)
        }
        Button(
            onClick = onScan,
            enabled = !state.completing,
            modifier = Modifier.fillMaxWidth(),
            colors = ButtonDefaults.buttonColors(
                containerColor = colors.surfaceVariant,
                contentColor = colors.onSurface,
                disabledContainerColor = colors.surfaceVariant.copy(alpha = 0.6f),
                disabledContentColor = colors.onSurfaceVariant,
            ),
        ) {
            Text("Scan QR code")
        }
    }
}
