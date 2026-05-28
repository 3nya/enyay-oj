## db schema

to run db, 
```
docker compose up -d
```
located at `localhost:8080`, login with user `root` and password.


### problems
```sql
CREATE TABLE problems (
    problem_id BIGINT PRIMARY KEY,
    problem_name VARCHAR(255) NOT NULL,
    runtime_ms BIGINT,
    memory_kb BIGINT
);
```

### submissions
```sql
CREATE TABLE submissions (
    submission_id BIGINT PRIMARY KEY,
    
    user_id BIGINT REFERENCES users(user_id),
    problem_id BIGINT REFERENCES problems(problem_id),
    
    verdict VARCHAR(255),
    runtime_ms BIGINT,
    memory_kb BIGINT,
    
    source_code VARCHAR(255),
    
    submitted_time TIMESTAMP
);
```

### users
```sql
CREATE TABLE users (
    user_id BIGINT PRIMARY KEY,
    user_name VARCHAR(255) NOT NULL
);
```