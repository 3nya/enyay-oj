# enyay-oj

Stack
- sql/postgres db, something (still deciding) backend/server, whatever frontend

Project plan
- [ ] Decide what functions are necessary
- [ ] Design backend (postgresql) schemas 
- [ ] Docker stuff for submitting code
- [ ] Set up backend db and server
- [ ] API endpoints 
- [ ] Slop some ok looking frontend

goals: get this working for 1 language


## functions

problems
- input/make new problem
- get problem

users submissions table
- get recent 20 submissions
- insert/submit problem
- get result of submission

user  
- get id, username, pfp

## db schema

problems table
- problem id
- problem title
- CPU/Time limit
- problem statement

user submissions
- user id
- problem id
- submission result (AC/WA/MLE/TLE/etc)
- code submitted

users 
- user id
- username
- profile picture