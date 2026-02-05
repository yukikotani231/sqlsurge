-- Invalid query - should fail

-- E0001: Table not found
SELECT * FROM nonexistent_table;

-- E0002: Column not found
SELECT user_id FROM users;

-- Column not found (with similar name suggestion)
SELECT naem FROM users;
