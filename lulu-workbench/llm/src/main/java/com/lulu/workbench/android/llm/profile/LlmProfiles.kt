package com.lulu.workbench.android.llm.profile

import com.lulu.workbench.android.llm.DEFAULT_LLM_ID
import com.lulu.workbench.android.llm.LLM_PRESETS
import com.lulu.workbench.android.llm.LlmActive
import com.lulu.workbench.android.llm.llmPreset
import com.lulu.workbench.android.storage.Storage

internal class LlmProfiles(
    private val storage: Storage,
) {
    fun migrateLegacy() {
        for (preset in LLM_PRESETS) {
            if (storage.read(profilePath(preset.id)) != null) continue
            val url = storage.read(splitPath(preset.id, "base_url"))?.decodeToString().orEmpty()
            val model =
                storage.read(splitPath(preset.id, "model"))?.decodeToString().orEmpty()
                    .ifEmpty {
                        if (preset.id == DEFAULT_LLM_ID) {
                            storage.read(LEGACY_MODEL_PATH)?.decodeToString().orEmpty()
                        } else {
                            ""
                        }
                    }
            if (url.isNotEmpty() || model.isNotEmpty()) {
                writeProfile(preset.id, url, model)
            }
            storage.delete(splitPath(preset.id, "base_url"))
            storage.delete(splitPath(preset.id, "model"))
        }
        val legacyKey = storage.getSecret(LEGACY_KEY)
        if (!legacyKey.isNullOrEmpty() && storage.getSecret(keyName(DEFAULT_LLM_ID)).isNullOrEmpty()) {
            storage.putSecret(keyName(DEFAULT_LLM_ID), legacyKey)
        }
    }

    fun selectedId(): String {
        val raw = storage.read(SELECTED_PATH)?.decodeToString()?.trim().orEmpty()
        return if (llmPreset(raw) != null) raw else DEFAULT_LLM_ID
    }

    fun select(id: String) {
        val preset = requireNotNull(llmPreset(id)) { "unknown llm id $id" }
        storage.write(SELECTED_PATH, preset.id.encodeToByteArray())
    }

    fun loadActive(): LlmActive {
        migrateLegacy()
        val preset = llmPreset(selectedId()) ?: llmPreset(DEFAULT_LLM_ID)!!
        val profile = readProfile(preset.id)
        return LlmActive(
            id = preset.id,
            label = preset.label,
            baseUrl = profile.baseUrl.ifBlank { preset.defaultBaseUrl },
            model = profile.model.ifBlank { preset.defaultModel },
            hasApiKey = !storage.getSecret(keyName(preset.id)).isNullOrBlank(),
        )
    }

    fun saveActive(baseUrl: String, model: String, apiKey: String) {
        val id = selectedId()
        requireNotNull(llmPreset(id)) { "unknown llm id $id" }
        writeProfile(id, baseUrl.trim(), model.trim())
        if (apiKey.isNotBlank()) {
            storage.putSecret(keyName(id), apiKey.trim())
        }
    }

    fun resetActive() {
        storage.delete(profilePath(selectedId()))
    }

    fun apiKey(id: String): String = storage.getSecret(keyName(id))?.trim().orEmpty()

    private fun readProfile(id: String): StoredProfile {
        val raw = storage.read(profilePath(id))?.decodeToString().orEmpty()
        if (raw.isBlank()) return StoredProfile("", "")
        return StoredProfile(
            baseUrl = jsonField(raw, "base_url"),
            model = jsonField(raw, "model"),
        )
    }

    private fun writeProfile(id: String, baseUrl: String, model: String) {
        storage.write(profilePath(id), encodeProfile(baseUrl, model).encodeToByteArray())
    }
}

private data class StoredProfile(
    val baseUrl: String,
    val model: String,
)

internal fun profilePath(id: String): String = "llm/$id"

internal fun keyName(id: String): String = "llm_key_$id"

private fun splitPath(id: String, field: String): String = "llm/$id/$field"

private fun encodeProfile(baseUrl: String, model: String): String =
    """{"base_url":"${escape(baseUrl)}","model":"${escape(model)}"}"""

private fun jsonField(json: String, key: String): String {
    val needle = "\"$key\""
    val at = json.indexOf(needle)
    if (at < 0) return ""
    val colon = json.indexOf(':', startIndex = at + needle.length)
    val quote = json.indexOf('"', startIndex = colon + 1)
    if (quote < 0) return ""
    return unescape(readJsonString(json, quote))
}

private fun readJsonString(source: String, openQuote: Int): String {
    val out = StringBuilder()
    var i = openQuote + 1
    while (i < source.length) {
        val c = source[i]
        if (c == '\\' && i + 1 < source.length) {
            out.append(source[i + 1])
            i += 2
            continue
        }
        if (c == '"') break
        out.append(c)
        i += 1
    }
    return out.toString()
}

private fun escape(value: String): String =
    value.replace("\\", "\\\\").replace("\"", "\\\"").replace("\n", "\\n")

private fun unescape(value: String): String =
    value.replace("\\n", "\n").replace("\\\"", "\"").replace("\\\\", "\\")

private const val SELECTED_PATH = "llm/selected"
private const val LEGACY_MODEL_PATH = "llm/model.txt"
private const val LEGACY_KEY = "llm_api_key"
