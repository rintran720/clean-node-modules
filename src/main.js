import { invoke } from "https://esm.sh/@tauri-apps/api@2/core";
import { listen } from "https://esm.sh/@tauri-apps/api@2/event";

const $ = (sel) => document.querySelector(sel);

const state = {
  rows: [],            // ProjectInfo[]
  selected: new Set(), // node_modules_path
  sortKey: "size",
  sortDir: "desc",
};

function fmtSize(n) {
  const units = ["B", "KB", "MB", "GB", "TB"];
  let i = 0; let v = n;
  while (v >= 1024 && i < units.length - 1) { v /= 1024; i++; }
  return `${v.toFixed(v < 10 && i > 0 ? 1 : 0)} ${units[i]}`;
}

function fmtDate(ms) {
  if (!ms) return "—";
  const d = new Date(ms);
  return d.toISOString().slice(0, 10);
}

function compare(a, b) {
  const k = state.sortKey;
  let av, bv;
  if (k === "size") { av = a.size_bytes; bv = b.size_bytes; }
  else if (k === "project_modified") { av = a.project_modified_ms; bv = b.project_modified_ms; }
  else if (k === "nm_modified") { av = a.nm_modified_ms; bv = b.nm_modified_ms; }
  else { av = a.project_path; bv = b.project_path; }
  if (av < bv) return state.sortDir === "asc" ? -1 : 1;
  if (av > bv) return state.sortDir === "asc" ? 1 : -1;
  return 0;
}

function render() {
  const body = $("#results-body");
  body.innerHTML = "";
  const sorted = [...state.rows].sort(compare);
  for (const r of sorted) {
    const tr = document.createElement("tr");
    const checked = state.selected.has(r.node_modules_path) ? "checked" : "";
    tr.innerHTML = `
      <td><input type="checkbox" data-path="${r.node_modules_path}" ${checked}></td>
      <td title="${r.project_path}">${r.project_path}</td>
      <td>${fmtSize(r.size_bytes)}</td>
      <td>${fmtDate(r.project_modified_ms)}</td>
      <td>${fmtDate(r.nm_modified_ms)}</td>
    `;
    body.appendChild(tr);
  }
  document.querySelectorAll("#results-table th[data-sort]").forEach((th) => {
    th.classList.toggle("active", th.dataset.sort === state.sortKey);
    th.classList.toggle("asc", state.sortDir === "asc");
  });
  updateSelectionSummary();
}

function updateSelectionSummary() {
  const sel = state.rows.filter((r) => state.selected.has(r.node_modules_path));
  const total = sel.reduce((s, r) => s + r.size_bytes, 0);
  $("#selection-summary").textContent = `${sel.length} selected · ${fmtSize(total)}`;
  $("#delete-btn").disabled = sel.length === 0;
}

$("#results-body").addEventListener("change", (e) => {
  const cb = e.target;
  if (cb.matches('input[type="checkbox"]')) {
    const p = cb.dataset.path;
    if (cb.checked) state.selected.add(p); else state.selected.delete(p);
    updateSelectionSummary();
  }
});

$("#select-all").addEventListener("change", (e) => {
  if (e.target.checked) state.rows.forEach((r) => state.selected.add(r.node_modules_path));
  else state.selected.clear();
  render();
});

document.querySelectorAll("#results-table th[data-sort]").forEach((th) => {
  th.addEventListener("click", () => {
    const k = th.dataset.sort;
    if (state.sortKey === k) state.sortDir = state.sortDir === "asc" ? "desc" : "asc";
    else { state.sortKey = k; state.sortDir = (k === "path") ? "asc" : "desc"; }
    render();
  });
});

$("#choose-btn").addEventListener("click", async () => {
  const root = await invoke("pick_root_folder");
  if (!root) return;
  startScan(root);
});

$("#cancel-btn").addEventListener("click", () => invoke("cancel_scan"));

async function startScan(root) {
  state.rows = [];
  state.selected.clear();
  $("#empty-state").hidden = true;
  $("#results").hidden = false;
  $("#footer").hidden = false;
  $("#cancel-btn").hidden = false;
  $("#status").textContent = `Scanning ${root}…`;
  render();
  await invoke("scan", { root });
}

await listen("scan:project", (event) => {
  state.rows.push(event.payload);
  render();
});

await listen("scan:progress", (event) => {
  const p = event.payload;
  $("#status").textContent =
    `Scanning… ${p.projects_found} projects, ${fmtSize(p.total_size_bytes)}`;
});

await listen("scan:done", (event) => {
  const d = event.payload;
  $("#cancel-btn").hidden = true;
  $("#status").textContent = d.cancelled
    ? `Cancelled · ${d.total_projects} projects, ${fmtSize(d.total_size_bytes)}`
    : `Done · ${d.total_projects} projects, ${fmtSize(d.total_size_bytes)}`;
});

$("#delete-btn").addEventListener("click", async () => {
  const sel = state.rows.filter((r) => state.selected.has(r.node_modules_path));
  if (sel.length === 0) return;
  const totalBytes = sel.reduce((s, r) => s + r.size_bytes, 0);
  const ok = confirm(
    `Move ${sel.length} node_modules folders (${fmtSize(totalBytes)}) to Trash?\n\n` +
    `You can restore them from your OS trash, or run \`npm install\` to recreate them.`
  );
  if (!ok) return;

  $("#delete-btn").disabled = true;
  const paths = sel.map((r) => r.node_modules_path);

  let results;
  try {
    results = await invoke("delete_many", { paths });
  } catch (err) {
    $("#status").textContent = `Delete failed: ${err}`;
    $("#delete-btn").disabled = false;
    return;
  }

  const okPaths = new Set(results.filter((r) => r.ok).map((r) => r.path));
  state.rows = state.rows.filter((r) => !okPaths.has(r.node_modules_path));
  okPaths.forEach((p) => state.selected.delete(p));

  const failed = results.filter((r) => !r.ok);
  if (failed.length === 0) {
    $("#status").textContent = `Deleted ${results.length} folders.`;
  } else {
    $("#status").textContent = `Deleted ${results.length - failed.length}, ${failed.length} failed.`;
    console.warn("Failed deletes:", failed);
  }
  render();
});
