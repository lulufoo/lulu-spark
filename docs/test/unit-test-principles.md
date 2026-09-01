# Unit Test Principles

1. **Test bodies do not live in production files.** Production code only declares that tests exist; implementations belong in the test tree.

2. **Tests belong to the domain under test, not to a test-kind taxonomy.** Contract or consistency checks with no matching production file still hang under that domain; they do not start a second tree.

3. **This layer covers unit tests only.** Other test kinds are a different layer and must not redefine this layout.

4. **Isolation is shared, not homemade.** Shared resources have one lifecycle, owned by shared fixtures. Tests must not mutate the environment, add private locks, or copy a private isolation kit.

5. **Commit-ready means the layout is intact and this layer is green.** Those two are gates, not advice.

## Rust

Lib unit tests only. Integration tests stay out of this layout. Process-level parallelism belongs to the runner; in-process serialization belongs to the fixtures.

## JS / TS

Gate, smoke, and integration checks must not redefine how unit tests are laid out.
