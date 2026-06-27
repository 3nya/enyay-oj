const app = document.querySelector("#app");
const headerLogin = document.querySelector("#login-header");
const navLinks = Array.from(document.querySelectorAll("[data-nav]"));
let statusRefreshTimer = null;
let renderToken = 0;

const state = {
  contests: [],
  problems: [],
  submissions: [],
  userSubmissions: [],
  authReady: false,
  authError: null,
  currentUser: null,
  dbUser: null,
};

const problemsPerPage = 10;

function escapeHtml(value) {
  return String(value ?? "")
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&#039;");
}

async function api(path, options = {}) {
  const response = await fetch(path, {
    headers: {
      "Content-Type": "application/json",
      ...(options.headers || {}),
    },
    ...options,
  });

  const text = await response.text();
  const body = text ? JSON.parse(text) : null;

  if (!response.ok) {
    throw new Error(body?.error || `${response.status} ${response.statusText}`);
  }

  return body;
}

function setActiveNav(route) {
  const key = route === "/" ? "home" : route.split("/")[1] || "home";
  navLinks.forEach((link) => {
    link.classList.toggle("active", link.dataset.nav === key);
  });
}

function navigate(path) {
  history.pushState({}, "", path);
  render();
}

function firebaseConfigReady() {
  const config = window.ENYAY_FIREBASE_CONFIG || {};
  return Boolean(config.apiKey && config.authDomain && config.projectId && config.appId);
}

async function initFirebaseAuth() {
  if (!window.firebase) {
    state.authReady = true;
    state.authError = "Firebase scripts did not load. Check your network connection.";
    render()
    return;
  }

  if (!firebaseConfigReady()) {
    state.authReady = true;
    state.authError = "Firebase config is missing. Fill in ENYAY_FIREBASE_CONFIG in index.html.";
    render()
    return;
  }

  try {
    if (!firebase.apps.length) {
      firebase.initializeApp(window.ENYAY_FIREBASE_CONFIG);
    }

    firebase.auth().onAuthStateChanged(async (user) => {
      state.authReady = true;
      state.authError = null;
      state.currentUser = user;
      state.problems = [];
      try{
        if(!user) state.dbUser = null;
        else await uidExists(user.uid);
      } catch(error){
        state.authError = error.message;
      }
      await renderHeaderLogin()
      render();
    });
  } catch (error) {
    state.authReady = true;
    state.authError = error.message;
    render()
  }
}

async function loadProblems() {
  if (state.problems.length) return state.problems;
  if(state.dbUser) state.problems = await api(`/problems/all/${state.dbUser.user_id}`);
  else state.problems = await api("/problems/all");
  return state.problems;
}

async function loadContests(){
  if(state.contests.length) return state.contests;
  if(state.dbUser) state.contests = await api(`/contests/recent/${state.dbUser.user_id}`);
  else state.contests = await api("/contests/recent");
  return state.contests;
}

async function loadSubmissions() {
  if (state.submissions.length) return state.submissions;
  state.submissions = await api("/submissions/recent");
  return state.submissions;
}
async function loadSubmissionsByUser(userId){
  if(state.userSubmissions.length) return state.userSubmissions;
  state.userSubmissions = await api(`/submissions/recent/${userId}`);
  return state.userSubmissions;
}

function getTimeZoneName(date) {
  const parts = new Intl.DateTimeFormat(undefined, {
    timeZoneName: "short",
  }).formatToParts(date);

  return parts.find((part) => part.type === "timeZoneName")?.value || "";
}

function formatDateTimeDetailed(time){
  return new Date(time).toLocaleString(undefined,{
    year: "numeric",
    month: "short",
    day: "numeric",
    hour: "numeric",
    minute: "2-digit",
    timeZoneName: "short"
  })
}

function formatDateTimeSum(time){
  const date = new Date(time);

  const year = date.getFullYear();
  const month = String(date.getMonth() + 1).padStart(2, "0");
  const day = String(date.getDate()).padStart(2, "0");
  const hour = String(date.getHours()).padStart(2, "0");
  const minute = String(date.getMinutes()).padStart(2, "0");
  const second = String(date.getSeconds()).padStart(2, "0");
  const zone = getTimeZoneName(date);

  return `${year}-${month}-${day} ${hour}:${minute}:${second} ${zone}`;
}

function renderLoading(label = "loading") {
  app.innerHTML = `<p class="status">${label}</p>`;
}

function renderError(error, token) {
  if(token != renderToken) return;

  app.innerHTML = `
    <section class="panel">
      <div class="panel-header">
        <h1 class="panel-title">request failed</h1>
      </div>
      <p class="status error" style="padding: 0 14px 14px;">${escapeHtml(error.message)}</p>
    </section>
  `;
}

async function renderHeaderLogin(){
  let dbUser = state.dbUser;

  headerLogin.innerHTML = `
    ${
      dbUser ?
      `<div class="header-login-row">
        <a class="header-login" href="/login" data-link data-nav="login">${escapeHtml(dbUser.user_name)}</a>
        <span>|</span>
        <a id = "header-logout" class="header-login">logout</a>
      </div>
      `
      : `<a class="header-login" href="/login" data-link data-nav="login">login</a>`
    }
  `
  if(dbUser){
    document.querySelector("#header-logout")?.addEventListener("click", async (event) => {
      event.preventDefault();
      try{
        await firebase.auth().signOut();
        state.userSubmissions = [];
        state.currentUser = null;
        state.dbUser = null;
        await renderHeaderLogin();
      } catch{
        navigate("/login");
      }
    });
  }
}

async function renderHome(token) {
  if(token != renderToken) return;

  renderLoading("loading homepage");
  const [problems, submissions] = await Promise.all([
    loadProblems().catch(() => []),
    loadSubmissions().catch(() => []),
  ]);

  const recentProblems = problems.slice(0, 4);
  const recentSubmissions = submissions.slice(0, 5);
  const rows = recentSubmissions.map( (submission) =>{
    let status = "error-color";
    if(submission.verdict === "AC") status = "success-color";
    else if(submission.verdict === "PENDING" || submission.verdict === "JUDGING") status = "pending-color";
    return {submission, status};
  });

  if(token != renderToken) return;

  app.innerHTML = `
    <section class="home-grid home-grid-single">
      <div class="panel">
        <div class="panel-header">
          <h1 class="panel-title">recent problems</h1>
          <a class="button secondary" href="/problemset" data-link>problemset</a>
        </div>
        <ul class="recent-list">
          ${
            recentProblems.length
              ? recentProblems
                  .map(
                    (problem) => `
                      <li>
                        <a href="/problemset/problem/${problem.problem_id}" data-link>${escapeHtml(problem.problem_name)}</a>
                        <span> ${escapeHtml(problem.runtime_ms)} ms, ${escapeHtml(problem.memory_mb)} MB, ${escapeHtml(problem.problem_rating)}</span>
                      </li>
                    `,
                  )
                  .join("")
              : `<li>No problems returned by the backend yet.</li>`
          }
        </ul>
      </div>
    </section>

    <section class="panel" style="margin-top: 18px;">
      <div class="panel-header">
        <h2 class="panel-title">recent submissions</h2>
      </div>
      <ul class="recent-list">
        ${
          rows.length
            ? rows
                .map(
                  ({submission, status}) => `
                    <li>
                      submission ${escapeHtml(submission.submission_id)}
                      <span> <span class = "${status}">${escapeHtml(submission.verdict)}</span> on problem ${escapeHtml(submission.problem_id)}</span>
                    </li>
                  `,
                )
                .join("")
            : `<li>No submissions returned by the backend yet.</li>`
        }
      </ul>
    </section>
  `;
}

async function renderContests(token){
  if(token != renderToken) return;
  
  renderLoading("loading contests");

  const contests = await loadContests();
  if(token != renderToken) return;
  const currentAndUpcoming = [];
  const pastContests = [];
  const currentTime = Date.now();
  for(let i = 0; i < contests.length; i++){
    let startTime = new Date(contests[i].start_time);
    if(currentTime <= startTime.getTime() || contests[i].is_active){
      currentAndUpcoming.push(contests[i]);
    } else pastContests.push(contests[i]);
  }

/*
This is a temporary UI for testing
*/
  app.innerHTML = `
  <section class="home-grid home-grid-single table-scroll">
    <div class="panel">
      <div class="panel-header">
        <h1 class="panel-title">Current or upcoming contests</h1>
    </div>
    <div>
      <table class="general-table contest-table" aria-label="Upcoming and active contests">
        <colgroup>
          <col style="width: 25%;">
          <col style="width: 20%;">
          <col style="width: 20%;">
          <col style="width: 20%;">
          <col style="width: 15%;">
        </colgroup>
        <thead>
          <tr>
            <th>Name</th>
            <th>Hosts</th>
            <th>Start</th>
            <th>End</th>
            <th></th>
          </tr>
        </thead>
        <tbody>
          ${
            currentAndUpcoming.length
              ? currentAndUpcoming
                  .map(
                    (contest) => `
                      <tr>
                        <td><a href="/contests/${contest.contest_id}" data-link>${escapeHtml(contest.contest_name)}</a></td>
                        <td>${escapeHtml(contest.host)}</td>
                        <td>${escapeHtml(formatDateTimeDetailed(contest.start_time))}</td>
                        <td>${escapeHtml(formatDateTimeDetailed(contest.end_time))}</td>
                        <td>
                          ${
                            contest.is_active
                              ? `<a href="/contests/${contest.contest_id}" data-link>enter</a>`
                              : contest.registered
                                ? `<span class="contest-badge registered">registered</span>`
                                : `<a href="/contests/${contest.contest_id}/registration" data-link>register</a>`
                          }
                          <span>
                            <span aria-hidden="true">&#128100;</span>
                              ${escapeHtml(contest.registered_count)}
                          </span>
                        </td>
                      </tr>
                    `,
                  )
                  .join("")
              : `<tr><td class="empty-row" colspan="5">No contests found.</td></tr>`
          }
        </tbody>
      </table>
      </div>
      </div>
  </section>
  <br> 
  <br>
  <section class="home-grid home-grid-single table-scroll">
    <div class="panel">
      <div class="panel-header">
        <h1 class="panel-title">Past contests</h1>
    </div>
    <div>
      <table class="general-table contest-table" aria-label="Past Contests">
        <colgroup>
          <col style="width: 25%;">
          <col style="width: 20%;">
          <col style="width: 20%;">
          <col style="width: 20%;">
          <col style="width: 15%;">
        </colgroup>
        <thead>
          <tr>
            <th>Name</th>
            <th>Hosts</th>
            <th>Start</th>
            <th>End</th>
            <th></th>
          </tr>
        </thead>
        <tbody>
          ${
            pastContests.length
              ? pastContests
                  .map(
                    (contest) => `
                      <tr>
                        <td>
                          <a href="/contests/${contest.contest_id}" data-link>${escapeHtml(contest.contest_name)}</a>
                          ${
                            contest.registered 
                              ? `<span class="success-color checkmark" aria-hidden="true">&#10003;</span>`
                              : ""
                          }
                        </td>
                        <td>${escapeHtml(contest.host)}</td>
                        <td>${escapeHtml(formatDateTimeDetailed(contest.start_time))}</td>
                        <td>${escapeHtml(formatDateTimeDetailed(contest.end_time))}</td>
                        <td><a href="/contests/${contest.contest_id}" data-link>view data</a> <span class="icon-person" aria-hidden="true">&#128100;</span> x ${escapeHtml(contest.registered_count)}</td>
                      </tr>
                    `,
                  )
                  .join("")
              : `<tr><td class="empty-row" colspan="5">No contests found.</td></tr>`
          }
        </tbody>
      </table>
      </div>
      </div>
  </section>
`;
}

async function renderRegistration(contest_id,token){
  if(token != renderToken) return;
  app.innerHTML = `
    <section class="panel">
      <div class="panel-header">
        <h1 class="panel-title">Contest ${escapeHtml(contest_id)} Registration</h1>
        <a class="button secondary" href="/contests" data-link>Contests</a>
      </div>
      <p id="registration-message" class = "pending-color" style="padding: 0 14px 14px;">registering...</p>
    </section>
  `;

  const message = document.querySelector("#registration-message");
  if(!state.currentUser){
    message.className = "status error";
    message.textContent = "you are not logged in!";
  } else if(!state.dbUser){
    message.className = "status error";
    message.textContent = "you must create a username!";
  } else{
    try{
      await api(`/contests/${contest_id}/registrations/${state.dbUser.user_id}`,{
        method: "POST",
      });
      message.className = "status success";
      message.textContent = "You are registered!";
    } catch(error){
      message.className = "status error";
      message.textContent = error;
    }
  }
}

async function renderProblemset(token) {
  if(token != renderToken) return;

  renderLoading("loading problemset");
  const problems = await loadProblems();
  const params = new URLSearchParams(window.location.search);
  const requestedPage = Number(params.get("page") || "1");
  const totalPages = Math.max(1, Math.ceil(problems.length / problemsPerPage));
  const currentPage = Math.min(Math.max(1, requestedPage || 1), totalPages);
  const start = (currentPage - 1) * problemsPerPage;
  const visibleProblems = problems.slice(start, start + problemsPerPage);
  const previousPage = Math.max(1, currentPage - 1);
  const nextPage = Math.min(totalPages, currentPage + 1);

  if(token != renderToken) return;

  app.innerHTML = `
    <section class="panel">
    <div class="table-scroll">
      <table class="general-table" aria-label="Problemset">
        <colgroup>
          <col style="width: 40%;">
          <col style="width: 20%;">
          <col style="width: 20%;">
          <col style="width: 20%;">
        </colgroup>
        <thead>
          <tr>
            <th>problem</th>
            <th>runtime</th>
            <th>memory</th>
            <th>rating</th>
          </tr>
        </thead>
        <tbody>
          ${
            visibleProblems.length
              ? visibleProblems
                  .map(
                    (problem) => `
                      <tr>
                        <td>
                          <a href="/problemset/problem/${problem.problem_id}" data-link>${escapeHtml(problem.problem_name)}</a>
                          ${
                            problem.accepted 
                              ? `<span class="success-color checkmark" aria-hidden="true">&#10003;</span>`
                              : ""
                          }
                        </td>
                        <td>${escapeHtml(problem.runtime_ms)} ms</td>
                        <td>${escapeHtml(problem.memory_mb)} MB</td>
                        <td>${escapeHtml(problem.problem_rating)}</td>
                      </tr>
                    `,
                  )
                  .join("")
              : `<tr><td class="empty-row" colspan="4">No problems found.</td></tr>`
          }
        </tbody>
      </table>
    </div>
    </section>
    <div class="page-number" aria-label="Problemset pagination">
      <a class="button secondary ${currentPage === 1 ? "disabled" : ""}" href="/problemset?page=${previousPage}" data-link>prev</a>
      <span>page ${currentPage} of ${totalPages}</span>
      <a class="button secondary ${currentPage === totalPages ? "disabled" : ""}" href="/problemset?page=${nextPage}" data-link>next</a>
    </div>
  `;
}

async function renderProblem(problemId, token){
  if(token != renderToken) return;

  renderLoading("loading problem")
  const problem = await findProblem(problemId);
  if(!problem){
    renderPlaceholder("Problem does not exist","",token);
    return;
  }

  const example = await findExample(problemId);

  if(token != renderToken) return;

    app.innerHTML = `
    <section class="general-layout">
      <div class="panel">
        <div class="panel-header">
          <h1 class="panel-title">${escapeHtml(problem.problem_name)}</h1>
          <a class="button secondary" href="/problemset" data-link>problemset</a>
        </div>
        <div class="general-summary">
            <p class="preformatted">${escapeHtml(problem.problem_statement)}</p>
            ${
              example 
              ? `<h1 class="panel-title">Example</h1>
            <br>
            <div class="panel">
              <div class="panel-header">
                <h1 class="panel-title">Input</h1>
                 <button class="button secondary" id="copy-input" type="button">copy</button>
              </div>
              <div class="general-summary">
                <p class="preformatted">${escapeHtml(example.input)}</p>
              </div>
            </div>
            <br>
            <div class="panel">
              <div class="panel-header">
                <h1 class="panel-title">Output</h1>
                <button class="button secondary" id="copy-output" type="button">copy</button>
              </div>
              <div class="general-summary">
                <p class="preformatted">${escapeHtml(example.solution)}</p>
              </div>
            </div>` : ""
            }
          </div>
          <div class="status" id="submit-status" role="status"></div>
        </div>
      </div>

      <aside class="panel">
        <div class="panel-header">
          <h2 class="panel-title">problem</h2>
        </div>
        <div class="general-summary">
          <dl>
            <dt>name</dt>
            <dd>${escapeHtml(problem.problem_name)}</dd>
            <dt>runtime</dt>
            <dd>${escapeHtml(problem.runtime_ms)} ms</dd>
            <dt>memory</dt>
            <dd>${escapeHtml(problem.memory_mb)} MB</dd>
            <dt>rating</dt>
            <dd>${escapeHtml(problem.problem_rating)}</dd>
            <dt>status</dt>
            ${
              problem.accepted 
              ?`<dd class="success-color">solved!</dd>`
              :`<dd class="error-color">unsolved</dd>`
            }
          </dl>
          <div class = "actions center-actions">
            <a class="button" href="/submit/${problem.problem_id}" data-link>submit</a>
          </div>
        </div>
      </aside>
    </section>
  `;
  if(example){
    document.querySelector("#copy-input")?.addEventListener("click", () => copyToBoard(example.input, "copy-input"));
    document.querySelector("#copy-output")?.addEventListener("click", () => copyToBoard(example.solution, "copy-output"));
  }
}

async function copyToBoard(value, buttonId) {
  const button = document.querySelector(`#${buttonId}`);
  if(!button) return;
  try {
    await navigator.clipboard.writeText(value);
    button.textContent = "copied"
    setTimeout(() =>{
      button.textContent = "copy"
    }, 1200);
  } catch (error) {
    button.textContent = "failed"
    setTimeout(() =>{
      button.textContent = "copy"
    },1200);
    return;
  }
}

async function findExample(problemId) {
  if(problemId) {
    try{
      return await api(`/problems/${problemId}/example`);
    } catch {
      return null;
    }
  }
}

async function findProblem(problemId) {
  if (problemId) {
    try {
      if(state.dbUser){
        return await api(`/problems/${problemId}/${state.dbUser.user_id}`);
      }
      else return await api(`/problems/${problemId}`);
    } catch {
      return null;
    }
  }

  const problems = await loadProblems();
  return problems[0] || null;
}

async function renderSubmit(problemId, token) {
  if(token != renderToken) return;

  renderLoading("loading submit page");
  const [problem, problems] = await Promise.all([
    findProblem(problemId),
    loadProblems().catch(() => []),
  ]);

  const selectedId = problem?.problem_id ?? problems[0]?.problem_id ?? "";

  if(token != renderToken) return;

  app.innerHTML = `
    <section class="submit-layout">
      <form class="panel" id="submission-form">
        <div class="panel-header">
          <h1 class="panel-title">submit solution</h1>
          <a class="button secondary" href="/problemset" data-link>problemset</a>
        </div>
        <div style="padding: 14px;">
          <div class="form-grid">
            <div class="field">
              <label for="problem-id">problem</label>
              <select id="problem-id" name="problem_id" required>
                ${
                  problems.length
                    ? problems
                        .map(
                          (item) => `
                            <option value="${item.problem_id}" ${Number(item.problem_id) === Number(selectedId) ? "selected" : ""}>
                              ${escapeHtml(item.problem_name)}
                            </option>
                          `,
                        )
                        .join("")
                    : `<option value="${escapeHtml(selectedId)}">problem ${escapeHtml(selectedId)}</option>`
                }
              </select>
            </div>
            <div class="field">
              <label for="language">language</label>
              <select id="language" name="language">
                <option value="c++20">c++20</option>
                <option value="python3">python3.12</option>
              </select>
            </div>
          </div>
          <div class="field">
            <label for="source-code">source code</label>
            <textarea id="source-code" name="source_code" spellcheck="false" required></textarea>
          </div>
          <div class="actions">
            <button class="button" type="submit">submit</button>
          </div>
          <div class="status" id="submit-status" role="status"></div>
        </div>
      </form>

      <aside class="panel">
        <div class="panel-header">
          <h2 class="panel-title">problem</h2>
        </div>
        <div class="general-summary">
          ${
            problem
              ? `
                <dl>
                  <dt>name</dt>
                  <dd>${escapeHtml(problem.problem_name)}</dd>
                  <dt>runtime</dt>
                  <dd>${escapeHtml(problem.runtime_ms)} ms</dd>
                  <dt>memory</dt>
                  <dd>${escapeHtml(problem.memory_mb)} MB</dd>
                  <dt>rating</dt>
                  <dd>${escapeHtml(problem.problem_rating)}</dd>
                </dl>
              `
              : `<p class="status">Select a problem to submit.</p>`
          }
        </div>
      </aside>
    </section>
  `;

  document.querySelector("#problem-id")?.addEventListener("change", (event) => {
    navigate(`/submit/${event.target.value}`);
  });
  document.querySelector("#submission-form")?.addEventListener("submit", submitSolution);
  const sourceCode = document.querySelector("#source-code");
  if(sourceCode){
    enableTabs(sourceCode);
  }
}

function enableTabs(textarea) {
  const tab = "    ";
  textarea.addEventListener("keydown",(event) => {
    if(event.key === "Tab") {
      event.preventDefault();
      const start = textarea.selectionStart;
      const end = textarea.selectionEnd;
      const value = textarea.value;
      textarea.value = value.slice(0,start) + tab + value.slice(end);
      textarea.selectionStart = textarea.selectionEnd = start+tab.length;
      return;
    } else if(event.key == "Backspace"){
      const start = textarea.selectionStart;
      const end = textarea.selectionEnd;
      if(start !== end || start < tab.length) return;
      const value = textarea.value;
      const selectedValue = value.slice(start - tab.length, start);
      if(selectedValue === tab){
        event.preventDefault();
        textarea.value = value.slice(0,start-tab.length) + value.slice(end);
        textarea.selectionStart = textarea.selectionEnd = start - tab.length;
      }
    }
  })
}

async function submitSolution(event) {
  const status = document.querySelector("#submit-status");
  event.preventDefault();

  const form = event.currentTarget;
  const button = form.querySelector("button[type='submit']");
  const data = new FormData(form);

  button.disabled = true;
  status.className = "status";
  status.textContent = "checking submission";

  try {
    if(!state.currentUser){
      status.textContent = "Please login";
      status.className = "status error";
      return;
    }
    const user = await uidExists(state.currentUser.uid);
    if(!user){
      status.textContent = "Please create a username";
      status.className = "status error";
      return;
    }

    status.textContent = "creating submission";
    const submission = await api("/submissions", {
      method: "POST",
      body: JSON.stringify({
        user_id: user.user_id,
        problem_id: Number(data.get("problem_id")),
        language: data.get("language"),
        source_code: data.get("source_code"),
      }),
    });
    
    state.userSubmissions = [];
    state.submissions = [];

    status.textContent = `submission ${submission.id} created, running judge`;
    navigate('/status/my');
  } catch (error) {
    status.className = "status error";
    status.textContent = error.message;
  } finally {
    button.disabled = false;
  }
}

async function renderStatus(token){
  if(token != renderToken) return;

  renderLoading("loading status");
  stopRefresh()
  const shouldPull = await renderTable(token);

  if(shouldPull){
    statusRefreshTimer = setInterval( async () => {
      const shouldContinue = await renderTable(token).catch((error) =>{
        console.error(error);
        return false;
      });
      if(!shouldContinue) stopRefresh();
    }, 3000);
  }
}

async function renderTable(token){
  if(token != renderToken){
    stopRefresh();
    return;
  }

  let myOnly = window.location.pathname === "/status/my"
  if(!myOnly) state.submissions = [];
  else state.userSubmissions = [];

  let submissions = null;
  if(myOnly && state.currentUser){
    const user = await uidExists(state.currentUser.uid);
    if(!user){
      navigate('/login/users');
      return;
    }
    submissions = await loadSubmissionsByUser(user.user_id);
  }
  else{
    submissions = await loadSubmissions();
    myOnly = false;
  }
  const params = new URLSearchParams(window.location.search);
  const requestedPage = Number(params.get("page") || "1");
  const totalPages = Math.max(1, Math.ceil(submissions.length / problemsPerPage));
  const currentPage = Math.min(Math.max(1, requestedPage || 1), totalPages);
  const start = (currentPage - 1) * problemsPerPage;
  const visibleSubmissions = submissions.slice(start, start + problemsPerPage);
  const previousPage = Math.max(1, currentPage - 1);
  const nextPage = Math.min(totalPages, currentPage + 1);
  const statusPath = myOnly ? "/status/my" : "/status"

  let myOnlyCheck = myOnly ? "checked" : "";

  const rows = visibleSubmissions.map( (submission) => {
    let status = "status error";
    if(submission.verdict === "AC") status = "status success";
    else if(submission.verdict === "PENDING" || submission.verdict === "JUDGING") status = "status pending";
    return {submission, status};
  })

  if(token != renderToken){
    stopRefresh();
    return;
  }

  app.innerHTML = `
    <section class="panel">
      <div class="status-toolbar">
        <label class="checkline">
          <input id="my-only" type="checkbox" ${myOnlyCheck}>
            my only
        </label>
      </div>
      <div class="table-scroll">
      <table class="general-table status-table" aria-label="Status">
        <colgroup>
          <col style="width: 8%;">
          <col style="width: 22%;">
          <col style="width: 18%;">
          <col style="width: 12%;">
          <col style="width: 12%;">
          <col style="width: 14%;">
          <col style="width: 14%;">
        </colgroup>
        <thead>
          <tr>
            <th>problem</th>
            <th>when</th>
            <th>who</th>
            <th>runtime</th>
            <th>memory</th>
            <th>language</th>
            <th>verdict</th>
          </tr>
        </thead>
        <tbody>
          ${
            rows.length
              ? rows
                  .map(
                    ({submission, status}) => `
                      <tr>
                        <td><a href="/problemset/problem/${submission.problem_id}" data-link>${escapeHtml(submission.problem_id)}</a></td>
                        <td>${escapeHtml(formatDateTimeSum(submission.submitted_time))}</td>
                        <td>${escapeHtml(submission.user_name || `user ${submission.user_id}`)}</td>
                        <td>${escapeHtml(submission.runtime_ms ?? "-")} ms</td>
                        <td>${escapeHtml(submission.memory_kb ?? "-")} KB</td>
                        <td>${escapeHtml(submission.language ?? "-")}</td>
                        <td class="${status}">${escapeHtml(submission.verdict)}</td>
                      </tr>
                    `,
                  )
                  .join("")
              : `<tr><td class="empty-row" colspan="7">No submissions found.</td></tr>`
          }
        </tbody>
      </table>
      </div>
    </section>
    <div class="page-number" aria-label="status pagination">
      <a class="button secondary ${currentPage === 1 ? "disabled" : ""}" href="${statusPath}?page=${previousPage}" data-link>prev</a>
      <span>page ${currentPage} of ${totalPages}</span>
      <a class="button secondary ${currentPage === totalPages ? "disabled" : ""}" href="${statusPath}?page=${nextPage}" data-link>next</a>
    </div>
  `;

  document.querySelector("#my-only").addEventListener("change", (event) => {
    if(event.target.checked){
      navigate("/status/my");
    } else{
      navigate("/status");
    }
  });
  return myOnly ? hasPending() : true;
}

function hasPending(){
  return state.userSubmissions.some((submission) => 
    (submission.verdict === "PENDING" || submission.verdict === "JUDGING")
  );
}

function stopRefresh(){
  if(statusRefreshTimer){
    clearInterval(statusRefreshTimer);
    statusRefreshTimer = null;
  }
}

function renderPlaceholder(title, body, token) {
  if(token != renderToken) return;

  app.innerHTML = `
    <section class="panel">
      <div class="panel-header">
        <h1 class="panel-title">${escapeHtml(title)}</h1>
      </div>
      <p class="status" style="padding: 0 14px 14px;">${escapeHtml(body)}</p>
    </section>
  `;
}

async function renderCreateUser(token){
  if(token != renderToken) return;

  if(!state.currentUser){
    navigate('/login');
    return;
  }

  const existing = await uidExists(state.currentUser.uid)
  if(existing){
    navigate(`/login`);
    return;
  }

  if(token != renderToken) return;

  app.innerHTML = `
      <section class="panel panel-username">
      <div class="panel-header">
        <h1 class="panel-title">Create Username</h1>
      </div>
      <form id = "create-user-form" class="input-fields">
        <label for="username-input">Username:</label>
        <input id="username-input" class= "input-field" name="username" spellcheck="false" required>
        <div style="padding: 14px 0px;">
          <button class="button" type="submit">create</button>
        </div>
        <p class="status" id="create-user-status"></p>
      </form>
    </section>
  `;
  document.querySelector("#create-user-form").addEventListener("submit",createUser);
}

async function createUser(event){
  event.preventDefault();
  let form = event.currentTarget
  const button = form.querySelector("button[type='submit']");
  const status= document.querySelector("#create-user-status");
  const data = new FormData(form);
  const username = data.get("username").trim();
  
  button.disabled = true;
  status.className = "status"
  status.textContent = "checking username";

  try{
    const userExists = await usernameExists(username);
    if(userExists){
      status.textContent = "username already exists";
      status.className = "status error"
      return;
    }

    await api("/users", {
      method: "POST",
      body: JSON.stringify({
        user_name: username,
        auth_uid: state.currentUser.uid,
      }),
    });
    navigate(`/login`);
  } catch(error){
    status.textContent = error.message;
    status.className = "status error"
  } finally{
    button.disabled = false;
  }
}

async function usernameExists(username){
  try{
    await api(`/users/by-name/${encodeURIComponent(username)}`)
    return true;
  } catch(error){
    if(error.message.includes("not found")){
      return false;
    }
    throw error;
  }
}

async function uidExists(uid){
  if(state.dbUser?.auth_uid === uid) return state.dbUser;
  try{
    let user = await api(`/users/by-uid/${encodeURIComponent(uid)}`)
    state.dbUser = user;
    return user;
  } catch(error){
    if(error.message.includes("not found")){
      state.dbUser = null;
      return false;
    }
    throw error;
  }
}

async function getUserById(id){
  try{
    let user = await api(`/users/${id}`);
    return user;
  } catch{
    return null;
  }
}

async function renderLogin(token) {
  if(token != renderToken) return;

  const user = state.currentUser;
  let dbUser = null;
  if (user){
    dbUser = await uidExists(user.uid);
    if(!dbUser){
      navigate('/login/users')
      return;
    }
  } 
  const displayName = dbUser?.user_name || user?.displayName || user?.email || "signed-in user";
  const photoUrl = user?.photoURL;

  if(token != renderToken) return;

  app.innerHTML = `
    <section class="login-layout">
      <div class="panel login-panel">
        <div class="panel-header">
          <h1 class="panel-title">login</h1>
          <a class="button secondary" href="/" data-link>home</a>
        </div>
        <div class="login-body">
          ${
            state.authError
              ? `<p class="status error">${escapeHtml(state.authError)}</p>`
              : user
                ? `
                  <div class="account-row">
                    ${
                      photoUrl
                        ? `<img class="avatar" src="${escapeHtml(photoUrl)}" alt="">`
                        : `<div class="avatar fallback" aria-hidden="true">${escapeHtml(displayName.charAt(0).toUpperCase())}</div>`
                    }
                    <div>
                      <p class="account-name">${escapeHtml(displayName)}</p>
                      <p class="account-email">${escapeHtml(user.email || "")}</p>
                    </div>
                  </div>
                  <div class="actions">
                    <button class="button secondary" id="copy-token" type="button">copy id token</button>
                    <button class="button" id="sign-out" type="button">sign out</button>
                  </div>
                  <p class="status" id="auth-status">signed in with Google</p>
                `
                : `
                  <p class="login-copy">Use your Google account to submit solutions and track judge usage.</p>
                  <button class="google-button" id="google-login" type="button" ${state.authReady ? "" : "disabled"}>
                    <span class="google-mark" aria-hidden="true">G</span>
                    <span>continue with Google</span>
                  </button>
                  <p class="status" id="auth-status">${state.authReady ? "not signed in" : "loading auth"}</p>
                `
          }
        </div>
      </div>
    </section>
  `;
  document.querySelector("#google-login")?.addEventListener("click", signInWithGoogle);
  document.querySelector("#sign-out")?.addEventListener("click", signOut);
  document.querySelector("#copy-token")?.addEventListener("click", copyIdToken);
}

async function signInWithGoogle() {
  const status = document.querySelector("#auth-status");

  try {
    status.textContent = "opening Google sign-in";
    const provider = new firebase.auth.GoogleAuthProvider();
    provider.setCustomParameters({ prompt: "select_account" });
    await firebase.auth().signInWithPopup(provider);
  } catch (error) {
    status.className = "status error";
    status.textContent = error.message;
  }
}

async function signOut() {
  const status = document.querySelector("#auth-status");

  try {
    status.textContent = "signing out";
    await firebase.auth().signOut();
    state.userSubmissions = [];
    state.problems = [];
    state.currentUser = null;
    state.dbUser = null;
  } catch (error) {
    status.className = "status error";
    status.textContent = error.message;
  }
}

async function copyIdToken() {
  const status = document.querySelector("#auth-status");

  try {
    const token = await state.currentUser.getIdToken();
    await navigator.clipboard.writeText(token);
    status.className = "status";
    status.textContent = "id token copied";
  } catch (error) {
    status.className = "status error";
    status.textContent = error.message;
  }
}

async function render() {
  const token = ++renderToken;

  const route = window.location.pathname;
  setActiveNav(route);
  if(!route.startsWith("/status")) stopRefresh()

  try {
    if (route === "/") {
      await renderHome(token);
    } else if(route === "/contests"){
      await renderContests(token)
    } else if (/^\/contests\/\d+\/registration$/.test(route)){
      await renderRegistration(route.split("/")[2],token);
    }else if (route === "/problemset") {
      await renderProblemset(token);
    } else if(route.startsWith("/problemset/problem/")){
      await renderProblem(route.split("/")[3],token);
    } else if (route === "/submit") {
      await renderSubmit(null,token);
    } else if (route.startsWith("/submit/")) {
      await renderSubmit(route.split("/")[2],token);
    } else if (route.startsWith("/status")) {
      await renderStatus(token);
    } else if (route === "/login") {
      await renderLogin(token);
    } else if (route === "/login/users"){
      await renderCreateUser(token);
    } else if (route === "/about") {
      renderPlaceholder("about", "Enyay OJ is a local online judge for testing submitted solutions.",token);
    } else {
      renderPlaceholder("not found", "That page does not exist.", token);
    }
  } catch (error) {
    renderError(error, token);
  }

  if(token == renderToken) app.focus({ preventScroll: true });
}

document.addEventListener("click", (event) => {
  const link = event.target.closest("a[data-link]");
  if (!link) return;
  if (link.classList.contains("disabled")) {
    event.preventDefault();
    return;
  }

  const url = new URL(link.href);
  if (url.origin !== window.location.origin) return;

  event.preventDefault();
  navigate(`${url.pathname}${url.search}`);
});

window.addEventListener("popstate", render);
renderLoading("loading");
initFirebaseAuth();
