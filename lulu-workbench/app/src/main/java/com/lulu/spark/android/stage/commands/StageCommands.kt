package com.lulu.spark.android.stage.commands

import com.lulu.spark.android.agent.facade.AgentFacade
import com.lulu.spark.android.agent.facade.WorkbenchRuntime
import com.lulu.spark.android.agent.tools.stage.StagedItem
import javax.inject.Inject

class StageCommands(
    private val listAll: () -> List<StagedItem>,
    private val getOne: (String) -> StagedItem?,
    private val save: (String, String, String) -> StagedItem?,
    private val remove: (String) -> Boolean,
) {
    constructor(agent: AgentFacade) : this(
        listAll = { agent.listStaged() },
        getOne = { agent.getStaged(it) },
        save = { id, title, body -> agent.updateStaged(id, title, body) },
        remove = { agent.deleteStaged(it) },
    )

    @Inject
    constructor(runtime: WorkbenchRuntime) : this(runtime.agent)

    fun list(): List<StagedItem> = listAll()

    fun get(id: String): StagedItem? = getOne(id)

    fun update(id: String, title: String, body: String): StagedItem? = save(id, title, body)

    fun delete(id: String): Boolean = remove(id)
}
