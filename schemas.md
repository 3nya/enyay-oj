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
    memory_mb BIGINT
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
    memory_mb BIGINT,
    
    source_code VARCHAR(255),
    language VARCHAR(255),
    
    submitted_time TIMESTAMP
);
```

### users
```sql
CREATE TABLE users (
    user_id BIGINT AUTO_INCREMENT PRIMARY KEY,
    user_name VARCHAR(255) UNIQUE NOT NULL
);
```