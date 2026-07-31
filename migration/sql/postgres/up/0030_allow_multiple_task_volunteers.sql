ALTER TABLE task_assignments DROP CONSTRAINT task_assignments_pkey;
ALTER TABLE task_assignments ADD CONSTRAINT task_assignments_pkey PRIMARY KEY (task_id, volunteer_user_id);
