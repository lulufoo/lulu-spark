//! Business-side expansion of the Host file-tool path fence at Binding Set.
//! The Agent loop only stores the resulting path lists.

use crate::agent::fence::{stored_path, PathFence};
use crate::agent::hard_deny::HardDenyList;
use crate::config::paths;
use crate::mcp_host::registry::SEEDED_BUSINESS_KEY;

/// Expand fence lists for a business key. Unknown keys yield `None`.
pub fn expand_for_business_key(key: &str) -> Option<PathFence> {
    if key != SEEDED_BUSINESS_KEY {
        return None;
    }
    let cache = stored_path(paths::cache_dir().ok()?);
    let deny = HardDenyList::spark_defaults()?;

    // SPARK_DATA_DIR is closed to the LLM: it reaches notes and knowledge only
    // through `get_note_file` / `get_knowledge_file` copies. Deny wins over any
    // staged grant, so this also closes write. No default read roots remain;
    // session scratch and staged files are added per turn.
    // Hard-deny roots come from HardDenyList — do not copy those paths here.
    Some(PathFence {
        read_allow: vec![],
        read_deny: deny.roots().to_vec(),
        write_allow: vec![],
        write_deny: vec![],
        scratch_parent: Some(cache.join("agent-workspace")),
    })
}

#[cfg(test)]
#[path = "../../unit-tests/agent/fence_spark_tests.rs"]
mod tests;
