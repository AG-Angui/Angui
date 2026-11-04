# Search-and-rescue experience schema

Migration `m0066_extend_knowledge_schema` supports SQLite, PostgreSQL and MySQL:

- `knowledge_items.external_id`: nullable source identifier such as `EXP-000001`. A non-unique index on base and identifier allows revision versions to share the identifier.
- `knowledge_terms.parent_id`: nullable category parent. New parents and their ancestors must be active categories in the same base; tags cannot have parents. The API does not reparent existing terms.
- `knowledge_terms.description`: category documentation, empty for existing records.
- `knowledge_import_batches.source_metadata_json`: source file version, generation notes and field descriptions, defaulting to `{}`.

Items select the most specific active category through `category_id`. New assignments reject categories with active children. The server writes the selected category name to the `category` snapshot. Unrelated draft edits retain that snapshot; legacy single-level categories remain supported.

Administrator create/update requests accept `external_id` and `metadata`. Metadata must be a JSON object with a serialized size at most 128 KiB. Administrator item lists and details return it; ordinary search and learner previews omit metadata and external identifiers. Updates preserve omitted metadata, replace it when an object is supplied, and clear it with `{}`. An empty external identifier clears it; omission or null preserves it on update. Correction drafts must explicitly submit any source fields they retain.

Suggested metadata keys: `applicable_scenarios`, `elderly_traits`, `trigger_signals`, `judgment`, `judgment_basis`, `recommended_actions`, `priority`, `exclusions`, `common_pitfalls`, `risk_notes`, `case_evidence`, `verification_note`, `contributor_name`, `original_interview`, `original_answer`, `attribution_note`. These keys are not mandatory. Missing information may be omitted or null; source wording can be preserved without inferring structured rules, verification or case links. Keep the original answer separate from the edited judgment. Content publication and empirical verification are separate concepts.

Nonempty metadata participates in content deduplication, allowing source-only corrections. Empty objects preserve existing CSV hash behavior. Metadata is not added to retrieval or chat context; reviewed material needed there belongs in the item body.

CSV preview retains its exact nine-column header and accepts an additional multipart text field `source_metadata`, containing a JSON object string (maximum 128 KiB). Preview, query and confirmation responses return `source_metadata`. This change does not implement the search-and-rescue JSON file parser or import the supplied records.

Rollback refuses to discard populated extension fields. Apply migrations using the existing deployment workflow; this code change does not modify a running business database. PostgreSQL and MySQL scripts require validation against their respective deployed database versions.
