package com.lulu.workbench.android.llm

data class LlmPreset(
    val id: String,
    val label: String,
    val defaultBaseUrl: String,
    val defaultModel: String,
)

data class LlmActive(
    val id: String,
    val label: String,
    val baseUrl: String,
    val model: String,
    val hasApiKey: Boolean,
)

fun llmCatalog(): List<LlmPreset> = LLM_PRESETS

fun llmPreset(id: String): LlmPreset? = LLM_PRESETS.firstOrNull { it.id == id }

internal val LLM_PRESETS: List<LlmPreset> =
    listOf(
        LlmPreset(
            id = "glm",
            label = "GLM",
            defaultBaseUrl = "https://open.bigmodel.cn/api/paas/v4",
            defaultModel = "",
        ),
        LlmPreset(
            id = "kimi",
            label = "Kimi",
            defaultBaseUrl = "https://api.moonshot.ai/v1",
            defaultModel = "kimi-k3",
        ),
        LlmPreset(
            id = "openai",
            label = "OpenAI",
            defaultBaseUrl = "https://api.openai.com/v1",
            defaultModel = "gpt-4o-mini",
        ),
    )

internal const val DEFAULT_LLM_ID = "glm"
