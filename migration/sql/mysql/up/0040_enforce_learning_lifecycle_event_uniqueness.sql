DELETE FROM learning_content_review_events
WHERE id NOT IN (
    SELECT retained_id FROM (
        SELECT MIN(id) AS retained_id
        FROM learning_content_review_events
        GROUP BY content_type, content_id, content_version, event_type
    ) AS retained_events
);
-- statement-break
CREATE UNIQUE INDEX uq_learning_content_review_event ON learning_content_review_events(content_type, content_id, content_version, event_type);
