# canchen

## database

- use some kind of blob storage, not just relational tables
    - becomes tricky to manage all problem data in only tables
    - ex. what if a problem wants images in problem statement?

- have problem authors provide a zip file with problem contents
- use some standardized method for rendering that zip file to web

- index problems via problem_id
    - want to separate problem data from testcases
    - since we don't need the testcases to render the problem on the webpage

problem_id ->
    - problem_data
        - name
        - runtime
        - memory
        - problem statement
        - images associated with problem statement
        - sample testcases
        - etc.
    - problem_testcases (hidden testcases)


## judge

- want to have one docker image per language?
    - target a *fixed* image instead of :latest
    - want consistency when rerunning code

- docker container should run the whole process of
    - installing required dependencies
    - getting problem data testcases from database
    - compiling and running the code
    - diffing output and reporting results (maybe as a .json?)
    - uploading results to database?

- API should just be "judge submission xxx"
- should have internal logic to figure out
    - what docker container to use based on the language
    - which problem testcase to download
