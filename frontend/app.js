const app = document.querySelector("#app");
const navLinks = Array.from(document.querySelectorAll("[data-nav]"));

const state = {
  problems: [],
  submissions: [],
};

const problemsPerPage = 10;

const starterCode = `#include <iostream>

int main() {
    std::ios::sync_with_stdio(false);
    std::cin.tie(nullptr);

    return 0;
}
`;

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

async function loadProblems() {
  if (state.problems.length) return state.problems;
  state.problems = await api("/problems/all");
  return state.problems;
}

async function loadSubmissions() {
  if (state.submissions.length) return state.submissions;
  state.submissions = await api("/submissions/recent");
  return state.submissions;
}

function renderLoading(label = "loading") {
  app.innerHTML = `<p class="status">${label}</p>`;
}

function renderError(error) {
  app.innerHTML = `
    <section class="panel">
      <div class="panel-header">
        <h1 class="panel-title">request failed</h1>
      </div>
      <p class="status error" style="padding: 0 14px 14px;">${escapeHtml(error.message)}</p>
    </section>
  `;
}

async function renderHome() {
  renderLoading("loading homepage");
  const [problems, submissions] = await Promise.all([
    loadProblems().catch(() => []),
    loadSubmissions().catch(() => []),
  ]);

  const recentProblems = problems.slice(0, 4);
  const recentSubmissions = submissions.slice(0, 5);

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
                        <a href="/submit/${problem.problem_id}" data-link>${escapeHtml(problem.problem_name)}</a>
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
          recentSubmissions.length
            ? recentSubmissions
                .map(
                  (submission) => `
                    <li>
                      submission ${escapeHtml(submission.submission_id)}
                      <span> ${escapeHtml(submission.verdict)} on problem ${escapeHtml(submission.problem_id)}</span>
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

async function renderProblemset() {
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

  app.innerHTML = `
    <section class="panel">
      <table class="problem-table" aria-label="Problemset">
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
                        <td><a href="/submit/${problem.problem_id}" data-link>${escapeHtml(problem.problem_name)}</a></td>
                        <td>${escapeHtml(problem.runtime_ms)} ms</td>
                        <td>${escapeHtml(problem.memory_mb)} MB</td>
                        <td>${escapeHtml(problem.problem_rating)}</td>
                      </tr>
                    `,
                  )
                  .join("")
              : `<tr><td class="empty-row" colspan="4">No problems found.</td></tr>`
          }
          ${visibleProblems.length ? `<tr><td class="empty-row" colspan="4"></td></tr>` : ""}
        </tbody>
      </table>
    </section>
    <div class="page-number" aria-label="Problemset pagination">
      <a class="button secondary ${currentPage === 1 ? "disabled" : ""}" href="/problemset?page=${previousPage}" data-link>prev</a>
      <span>page ${currentPage} of ${totalPages}</span>
      <a class="button secondary ${currentPage === totalPages ? "disabled" : ""}" href="/problemset?page=${nextPage}" data-link>next</a>
    </div>
  `;
}

async function findProblem(problemId) {
  if (problemId) {
    try {
      return await api(`/problems/${problemId}`);
    } catch {
      return null;
    }
  }

  const problems = await loadProblems();
  return problems[0] || null;
}

async function renderSubmit(problemId) {
  renderLoading("loading submit page");
  const [problem, problems] = await Promise.all([
    findProblem(problemId),
    loadProblems().catch(() => []),
  ]);

  const selectedId = problem?.problem_id ?? problems[0]?.problem_id ?? "";

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
              <label for="user-id">user id</label>
              <input id="user-id" name="user_id" type="number" min="1" value="1" required>
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
            <label class="checkline">
              <input id="run-judge" type="checkbox" checked>
              run judge
            </label>
          </div>
          <div class="status" id="submit-status" role="status"></div>
        </div>
      </form>

      <aside class="panel">
        <div class="panel-header">
          <h2 class="panel-title">problem</h2>
        </div>
        <div class="problem-summary">
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
  event.preventDefault();

  const form = event.currentTarget;
  const button = form.querySelector("button[type='submit']");
  const status = document.querySelector("#submit-status");
  const runJudge = document.querySelector("#run-judge").checked;
  const data = new FormData(form);

  button.disabled = true;
  status.className = "status";
  status.textContent = "creating submission";

  try {
    const submission = await api("/submissions", {
      method: "POST",
      body: JSON.stringify({
        user_id: Number(data.get("user_id")),
        problem_id: Number(data.get("problem_id")),
        verdict: "PENDING",
        runtime_ms: null,
        memory_kb: null,
        language: data.get("language"),
        source_code: data.get("source_code"),
      }),
    });

    state.submissions = [];

    if (runJudge) {
      status.textContent = `submission ${submission.id} created, running judge`;
      await api(`/submissions/${submission.id}/judge`, { method: "POST" });
      status.textContent = `submission ${submission.id} created and judge run completed`;
    } else {
      status.textContent = `submission ${submission.id} created`;
    }
  } catch (error) {
    status.className = "status error";
    status.textContent = error.message;
  } finally {
    button.disabled = false;
  }
}

function renderPlaceholder(title, body) {
  app.innerHTML = `
    <section class="panel">
      <div class="panel-header">
        <h1 class="panel-title">${escapeHtml(title)}</h1>
      </div>
      <p class="status" style="padding: 0 14px 14px;">${escapeHtml(body)}</p>
    </section>
  `;
}

async function render() {
  const route = window.location.pathname;
  setActiveNav(route);

  try {
    if (route === "/") {
      await renderHome();
    } else if (route === "/problemset") {
      await renderProblemset();
    } else if (route === "/submit") {
      await renderSubmit(null);
    } else if (route.startsWith("/submit/")) {
      await renderSubmit(route.split("/")[2]);
    } else if (route === "/users-page") {
      renderPlaceholder("users", "User browsing is not wired yet.");
    } else if (route === "/about") {
      renderPlaceholder("about", "Enyay OJ is a local online judge for testing submitted solutions.");
    } else {
      renderPlaceholder("not found", "That page does not exist.");
    }
  } catch (error) {
    renderError(error);
  }

  app.focus({ preventScroll: true });
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
render();
