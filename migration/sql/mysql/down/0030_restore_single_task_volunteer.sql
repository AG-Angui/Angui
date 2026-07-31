DELETE a FROM task_assignments a JOIN task_assignments b ON a.task_id = b.task_id AND a.volunteer_user_id > b.volunteer_user_id;
ALTER TABLE task_assignments DROP PRIMARY KEY, ADD PRIMARY KEY (task_id);
