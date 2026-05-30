## db schema

to run db, 
```
docker compose up -d
```
located at `localhost:8080`, login with user `root` and password.


### problems
```sql
CREATE TABLE problems (
    problem_id BIGINT PRIMARY KEY AUTO_INCREMENT,
    problem_name VARCHAR(255) NOT NULL,
    runtime_ms BIGINT,
    memory_kb BIGINT
);
```

### submissions
```sql
CREATE TABLE submissions (
    submission_id BIGINT PRIMARY KEY AUTO_INCREMENT,

    user_id BIGINT NOT NULL,
    problem_id BIGINT NOT NULL,

    verdict VARCHAR(50) NOT NULL,
    runtime_ms BIGINT,
    memory_kb BIGINT,

    language VARCHAR(50),

    source_code TEXT NOT NULL,

    submitted_time TIMESTAMP DEFAULT CURRENT_TIMESTAMP,

    FOREIGN KEY (user_id) REFERENCES users(user_id),
    FOREIGN KEY (problem_id) REFERENCES problems(problem_id)
);
```

### users
```sql
CREATE TABLE users (
    user_id BIGINT PRIMARY KEY AUTO_INCREMENT,
    user_name VARCHAR(255) NOT NULL
);
```