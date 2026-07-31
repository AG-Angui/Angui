ALTER TABLE task_assignments DROP PRIMARY KEY, ADD PRIMARY KEY (task_id, volunteer_user_id);
