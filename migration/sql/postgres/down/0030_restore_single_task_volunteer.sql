DELETE FROM task_assignments a USING task_assignments b WHERE a.task_id = b.task_id AND a.volunteer_user_id > b.volunteer_user_id;
ALTER TABLE task_assignments DROP CONSTRAINT task_assignments_pkey;
ALTER TABLE task_assignments ADD CONSTRAINT task_assignments_pkey PRIMARY KEY (task_id);
