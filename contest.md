### Contest
```sql
CREATE TABLE contests (
    contest_id BIGINT PRIMARY KEY AUTO_INCREMENT,
    contest_name VARCHAR(255) UNIQUE NOT NULL,
    host VARCHAR(255) NOT NULL,
    start_time TIMESTAMP NOT NULL, 
    end_time TIMESTAMP NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT FALSE
);
```

### Registration
```sql
CREATE TABLE contest_registrations(
    contest_id BIGINT NOT NULL,
    user_id BIGINT NOT NULL,
    registered_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    points INT NOT NULL DEFAULT 0,
    penalty INT NOT NULL DEFAULT 0,
     
    PRIMARY KEY(contest_id, user_id),
    FOREIGN KEY(contest_id) REFERENCES contests(contest_id),
    FOREIGN KEY(user_id) REFERENCES users(user_id)
);
```

### Probelms
```sql
CREATE TABLE contest_problems(
    contest_id BIGINT NOT NULL,
    problem_id BIGINT NOT NULL,
    problem_order VARCHAR(1) NOT NULL,

    PRIMARY KEY(contest_id, problem_id),
    UNIQUE(contest_id, problem_order),
    FOREIGN KEY(contest_id) REFERENCES contests(contest_id),
    FOREIGN KEY(problem_id) REFERENCES problems(problem_id)
);
```

### Submissions
```sql
ALTER TABLE submissions ADD contest_id BIGINT;
ALTER TABLE submissions ADD FOREIGN KEY (contest_id) REFERENCES contests(contest_id);
```
### Problems
```sql
ALTER TABLE problems ADD is_public BOOLEAN NOT NULL DEFAULT TRUE;
```

### Indexes
```sql
CREATE INDEX idx_submissions_contest_user_problem_verdict
ON submissions (contest_id, user_id, problem_id, verdict);

CREATE INDEX idx_contest_rankings
ON contest_registrations (contest_id, points DESC, penalty ASC);
```