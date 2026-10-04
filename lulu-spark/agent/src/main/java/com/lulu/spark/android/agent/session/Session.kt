package com.lulu.spark.android.agent.session

import com.lulu.spark.android.storage.Storage
import java.util.UUID

data class SessionId(val value: String) {
    init {
        require(value.isNotBlank()) { "session id is blank" }
    }
}

data class SessionRoots(
    val historyPath: String,
    val toolPath: String,
)

class SessionRegistry(
    private val storage: Storage,
) {
    private val ids = linkedSetOf<SessionId>()

    init {
        ids.addAll(readIndex())
    }

    fun create(): SessionId {
        val id = SessionId("sess_" + UUID.randomUUID().toString().replace("-", "").take(12))
        ids.add(id)
        writeIndex()
        storage.write(roots(id).historyPath, "[]".encodeToByteArray())
        return id
    }

    fun list(): List<SessionId> = ids.toList()

    fun delete(id: SessionId) {
        if (!ids.remove(id)) return
        writeIndex()
        storage.list("sessions/${id.value}").forEach { storage.delete(it) }
    }

    fun roots(id: SessionId): SessionRoots =
        SessionRoots(
            historyPath = "sessions/${id.value}/history",
            toolPath = "sessions/${id.value}/tools",
        )

    fun loadTurns(id: SessionId): List<HistoryTurn> {
        val raw = storage.read(roots(id).historyPath)?.decodeToString().orEmpty()
        return decodeTurns(raw)
    }

    fun saveTurns(id: SessionId, turns: List<HistoryTurn>) {
        storage.write(roots(id).historyPath, encodeTurns(turns).encodeToByteArray())
    }

    private fun readIndex(): List<SessionId> {
        val raw = storage.read(INDEX_PATH)?.decodeToString().orEmpty()
        if (raw.isBlank()) return emptyList()
        return raw
            .removePrefix("[")
            .removeSuffix("]")
            .split(",")
            .map { it.trim().trim('"') }
            .filter { it.isNotEmpty() }
            .map { SessionId(it) }
    }

    private fun writeIndex() {
        val body = ids.joinToString(",", "[", "]") { "\"${it.value}\"" }
        storage.write(INDEX_PATH, body.encodeToByteArray())
    }
}

private const val INDEX_PATH = "sessions/index"
