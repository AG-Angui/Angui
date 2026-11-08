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

CSV preview accepts the original nine-column header (`knowledge_base_id,title,content,summary,category,keywords,source_name,source_url,visibility`) or that header with both `external_id,metadata` appended, in that order. `metadata` must contain a JSON object (maximum 128 KiB); an empty cell defaults to `{}`. Preview and confirmation preserve the identifier and metadata, and include metadata in duplicate detection. Older nine-column files and pending previews remain supported.

For experience records, keep the identifier in `external_id`, classification in `category`, and a concise body in `content`. The export uses metadata keys `primary_category`, `secondary_category` and `source_fields` (the complete original record). The administrator table maps all 21 source fields to separate columns, including judgment, recommended actions, priority, evidence and original answers. Long cells expand on click. Generated titles display a short heading, while their complete source title remains available in a tooltip. Previously imported labeled records are split for display without rewriting stored content.

The administrator page uses `GET /api/admin/knowledge-bases/{base_id}/items/page?page=1&page_size=20`. It counts and fetches one ordered database page, with allowed sizes 10, 20, 50 and 100. Pages use updated time descending and ID ascending for a stable tie-breaker. A size above 20 gives the table a 640 px scrollable viewport. The original unpaginated endpoint remains for existing clients.

CSV preview returns original `raw_data` cells as well as normalized data and validation errors. The UI provides paginated temporary editing; edits must be re-previewed and validated before confirming the replacement batch. Source files remain unchanged. UTF-8 BOM is accepted. Confirming a batch inserts valid rows directly with `published` status; no de-identification/review/version actions are shown in this knowledge table. This does not change the separate learning-resource/question governance workflow or retroactively publish old records.

An additional multipart text field `source_metadata` accepts file-level information as a JSON object string (maximum 128 KiB). Preview, query and confirmation responses return `source_metadata`. No source records are imported by this code change.

Rollback refuses to discard populated extension fields. Apply migrations using the existing deployment workflow; this code change does not modify a running business database. PostgreSQL and MySQL scripts require validation against their respective deployed database versions.
