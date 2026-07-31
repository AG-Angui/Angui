ALTER TABLE task_operation_idempotency ADD COLUMN request_fingerprint VARCHAR(64) NOT NULL DEFAULT '';
