package com.lulu.workbench.android.agent.tools.fs

import com.lulu.workbench.android.agent.tools.jsonIntField
import com.lulu.workbench.android.agent.tools.jsonStringField
import com.lulu.workbench.android.llm.LlmToolDef
import com.lulu.workbench.android.storage.Storage
import java.util.regex.Pattern
import java.util.regex.PatternSyntaxException

class FsTools {
    val names: List<String> = listOf("read", "grep", "write", "edit")

    fun definitions(): List<LlmToolDef> =
        listOf(
            LlmToolDef("read", "Read a text file under the tool session root.", READ_PARAMS),
            LlmToolDef("grep", "Search file contents under the tool session root.", GREP_PARAMS),
            LlmToolDef("write", "Create or overwrite a text file under the tool session root.", WRITE_PARAMS),
            LlmToolDef("edit", "Replace exact text once in a file under the tool session root.", EDIT_PARAMS),
        )

    fun call(name: String, arguments: String, toolRoot: String, storage: Storage): String =
        try {
            when (name) {
                "read" -> read(arguments, toolRoot, storage)
                "grep" -> grep(arguments, toolRoot, storage)
                "write" -> write(arguments, toolRoot, storage)
                "edit" -> edit(arguments, toolRoot, storage)
                else -> "unknown file tool '$name'"
            }
        } catch (error: IllegalArgumentException) {
            error.message ?: "invalid arguments"
        }

    private fun read(arguments: String, toolRoot: String, storage: Storage): String {
        val rawPath = jsonStringField(arguments, "path") ?: return "missing path"
        val path = fencePath(rawPath, toolRoot)
        val bytes = storage.read(path) ?: return "file not found"
        val lines = bytes.decodeToString().lines()
        val offset = (jsonIntField(arguments, "offset") ?: 1).coerceAtLeast(1)
        val limit = (jsonIntField(arguments, "limit") ?: READ_DEFAULT_LIMIT).coerceAtLeast(1)
        val start = (offset - 1).coerceAtMost(lines.size)
        val end = (start + limit).coerceAtMost(lines.size)
        if (start >= end) return "File is empty."
        return lines.subList(start, end).mapIndexed { i, line -> "${start + i + 1}:$line" }.joinToString("\n")
    }

    private fun grep(arguments: String, toolRoot: String, storage: Storage): String {
        val pattern = jsonStringField(arguments, "pattern") ?: return "missing pattern"
        val regex = try {
            Pattern.compile(pattern)
        } catch (error: PatternSyntaxException) {
            return "invalid pattern: ${error.message}"
        }
        val rawPath = jsonStringField(arguments, "path")
        val files = if (rawPath == null) {
            storage.list(toolRoot)
        } else {
            val path = fencePath(rawPath, toolRoot)
            storage.list(path).ifEmpty { if (storage.read(path) != null) listOf(path) else emptyList() }
        }
        val matches = mutableListOf<String>()
        var seen = 0
        for (file in files) {
            if (matches.size >= GREP_MAX_MATCHES || seen >= GREP_MAX_FILES) break
            if (!file.startsWith("$toolRoot/") && file != toolRoot) continue
            val bytes = storage.read(file) ?: continue
            seen += 1
            if (bytes.size > GREP_MAX_FILE_BYTES) continue
            bytes.decodeToString().lines().forEachIndexed { index, line ->
                if (matches.size < GREP_MAX_MATCHES && regex.matcher(line).find()) {
                    matches.add("$file:${index + 1}:$line")
                }
            }
        }
        return if (matches.isEmpty()) "No matches." else matches.joinToString("\n")
    }

    private fun write(arguments: String, toolRoot: String, storage: Storage): String {
        val rawPath = jsonStringField(arguments, "path") ?: return "missing path"
        val path = fencePath(rawPath, toolRoot)
        val content = jsonStringField(arguments, "content") ?: return "missing content"
        storage.write(path, content.encodeToByteArray())
        return "Wrote $path"
    }

    private fun edit(arguments: String, toolRoot: String, storage: Storage): String {
        val rawPath = jsonStringField(arguments, "path") ?: return "missing path"
        val path = fencePath(rawPath, toolRoot)
        val oldText = jsonStringField(arguments, "old_text") ?: return "missing old_text"
        val newText = jsonStringField(arguments, "new_text") ?: return "missing new_text"
        if (oldText.isEmpty()) return "old_text must not be empty"
        val current = storage.read(path)?.decodeToString() ?: return "file not found"
        val count = Regex.fromLiteral(oldText).findAll(current).count()
        if (count == 0) return "old_text not found"
        if (count > 1) return "old_text matched more than once"
        storage.write(path, current.replaceFirst(oldText, newText).encodeToByteArray())
        return "Edited $path"
    }
}

internal fun fencePath(path: String, toolRoot: String): String {
    val normalized = path.replace('\\', '/').trim()
    val joined =
        if (normalized == toolRoot || normalized.startsWith("$toolRoot/")) {
            normalized
        } else {
            "$toolRoot/${normalized.removePrefix("/").trimStart('/')}"
        }
    require(joined.split('/').none { it == ".." || it == "." }) {
        "path is outside the tool session root"
    }
    require(joined == toolRoot || joined.startsWith("$toolRoot/")) {
        "path is outside the tool session root"
    }
    return joined
}

private const val READ_DEFAULT_LIMIT = 2000
private const val GREP_MAX_MATCHES = 80
private const val GREP_MAX_FILES = 2000
private const val GREP_MAX_FILE_BYTES = 1_000_000

private const val READ_PARAMS =
    """{"type":"object","properties":{"path":{"type":"string"},"offset":{"type":"integer"},"limit":{"type":"integer"}},"required":["path"]}"""
private const val GREP_PARAMS =
    """{"type":"object","properties":{"pattern":{"type":"string"},"path":{"type":"string"}},"required":["pattern"]}"""
private const val WRITE_PARAMS =
    """{"type":"object","properties":{"path":{"type":"string"},"content":{"type":"string"}},"required":["path","content"]}"""
private const val EDIT_PARAMS =
    """{"type":"object","properties":{"path":{"type":"string"},"old_text":{"type":"string"},"new_text":{"type":"string"}},"required":["path","old_text","new_text"]}"""
