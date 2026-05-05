import { invoke } from "https://esm.sh/@tauri-apps/api@2/core";
import { listen } from "https://esm.sh/@tauri-apps/api@2/event";

const $ = (sel) => document.querySelector(sel);

const state = {
  rows: [],            // ProjectInfo[]
  selected: new Set(), // node_modules_path
  failed: new Map(),   // node_modules_path -> error message
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
    tr.dataset.path = r.node_modules_path;
    const checked = state.selected.has(r.node_modules_path) ? "checked" : "";
    if (state.failed.has(r.node_modules_path)) {
      tr.classList.add("failed");
      tr.title = state.failed.get(r.node_modules_path);
    }
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
  $("#delete-permanent-btn").disabled = sel.length === 0;
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

async function chooseAndScan() {
  const root = await invoke("pick_root_folder");
  if (!root) return;
  await invoke("cancel_scan");
  startScan(root);
}

$("#choose-btn").addEventListener("click", chooseAndScan);
$("#new-scan-btn").addEventListener("click", chooseAndScan);

$("#cancel-btn").addEventListener("click", () => invoke("cancel_scan"));

async function startScan(root) {
  state.rows = [];
  state.selected.clear();
  state.failed.clear();
  $("#delete-progress").hidden = true;
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

function findRowByPath(path) {
  for (const tr of document.querySelectorAll("#results-body tr")) {
    if (tr.dataset.path === path) return tr;
  }
  return null;
}

function handleDeleteStart({ index, total, path }) {
  $("#delete-progress").hidden = false;
  $("#delete-progress-bar").max = total;
  $("#delete-progress-bar").value = index - 1;
  $("#delete-progress-count").textContent = `${index} / ${total}`;
  $("#delete-progress-path").textContent = path;

  const tr = findRowByPath(path);
  if (tr) {
    tr.classList.add("deleting");
    tr.scrollIntoView({ block: "nearest", behavior: "smooth" });
  }
}

function handleDeleteProgress({ index, total, result }) {
  $("#delete-progress-bar").value = index;
  $("#status").textContent = `Deleting ${index}/${total}…`;

  const tr = findRowByPath(result.path);
  if (!tr) return;
  tr.classList.remove("deleting");
  if (result.ok) {
    tr.classList.add("removing");
    tr.addEventListener("animationend", () => tr.remove(), { once: true });
  } else {
    tr.classList.add("failed");
    tr.title = result.error ?? "Delete failed";
  }
}

const _delQ = [];
let _delDraining = false;
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

async function drainDeleteQueue() {
  if (_delDraining) return;
  _delDraining = true;
  while (_delQ.length) {
    const evt = _delQ.shift();
    if (evt.type === "start") {
      handleDeleteStart(evt.payload);
      await sleep(90);
    } else {
      handleDeleteProgress(evt.payload);
      await sleep(20);
    }
  }
  _delDraining = false;
}

await listen("delete:item-start", (event) => {
  _delQ.push({ type: "start", payload: event.payload });
  drainDeleteQueue();
});

await listen("delete:progress", (event) => {
  _delQ.push({ type: "progress", payload: event.payload });
  drainDeleteQueue();
});

function askConfirm({ title, summary, warning, requireType, okLabel, okDanger }) {
  const modal = $("#confirm-modal");
  const input = $("#confirm-type-input");
  const okBtn = $("#confirm-ok");
  const cancelBtn = $("#confirm-cancel");
  const warnEl = $("#confirm-warning");
  const typeLabel = $("#confirm-type-label");

  $("#confirm-title").textContent = title;
  $("#confirm-summary").textContent = summary;
  warnEl.textContent = warning ?? "";
  warnEl.hidden = !warning;
  typeLabel.hidden = !requireType;
  okBtn.textContent = okLabel ?? "Confirm";
  okBtn.classList.toggle("danger", !!okDanger);
  input.value = "";
  okBtn.disabled = !!requireType;

  return new Promise((resolve) => {
    const onInput = () => { okBtn.disabled = requireType && input.value !== "DELETE"; };
    const onCancel = (e) => { e.preventDefault(); modal.close("cancel"); };
    const onClose = () => {
      input.removeEventListener("input", onInput);
      cancelBtn.removeEventListener("click", onCancel);
      modal.removeEventListener("close", onClose);
      const ok = modal.returnValue === "ok" && (!requireType || input.value === "DELETE");
      resolve(ok);
    };
    input.addEventListener("input", onInput);
    cancelBtn.addEventListener("click", onCancel);
    modal.addEventListener("close", onClose);
    modal.returnValue = "";
    modal.showModal();
    if (requireType) input.focus();
  });
}

async function runDelete({ command, confirmFn }) {
  const sel = state.rows.filter((r) => state.selected.has(r.node_modules_path));
  if (sel.length === 0) return;
  if (!(await confirmFn(sel))) return;

  $("#delete-btn").disabled = true;
  $("#delete-permanent-btn").disabled = true;
  const paths = sel.map((r) => r.node_modules_path);

  $("#delete-progress").hidden = false;
  $("#delete-progress-bar").max = paths.length;
  $("#delete-progress-bar").value = 0;
  $("#delete-progress-count").textContent = `0 / ${paths.length}`;
  $("#delete-progress-path").textContent = "Preparing…";
  $("#status").textContent = `Deleting 0/${paths.length}…`;

  let results;
  try {
    results = await invoke(command, { paths });
  } catch (err) {
    $("#status").textContent = `Delete failed: ${err}`;
    $("#delete-progress").hidden = true;
    updateSelectionSummary();
    return;
  }

  const okPaths = new Set(results.filter((r) => r.ok).map((r) => r.path));
  state.rows = state.rows.filter((r) => !okPaths.has(r.node_modules_path));
  okPaths.forEach((p) => state.selected.delete(p));
  for (const r of results.filter((r) => !r.ok)) {
    state.failed.set(r.path, r.error ?? "Delete failed");
  }

  const failed = results.filter((r) => !r.ok);
  if (failed.length === 0) {
    $("#status").textContent = `Deleted ${results.length} folders.`;
  } else {
    $("#status").textContent = `Deleted ${results.length - failed.length}, ${failed.length} failed.`;
    console.warn("Failed deletes:", failed);
  }

  while (_delQ.length || _delDraining) await sleep(50);
  setTimeout(() => { $("#delete-progress").hidden = true; }, 500);
  updateSelectionSummary();
}

$("#delete-btn").addEventListener("click", () =>
  runDelete({
    command: "delete_many",
    confirmFn: (sel) => {
      const totalBytes = sel.reduce((s, r) => s + r.size_bytes, 0);
      return askConfirm({
        title: "Move to Trash?",
        summary: `${sel.length} node_modules folders (${fmtSize(totalBytes)}) will be moved to the OS Trash. You can restore them from there, or run \`npm install\` to recreate them.`,
        okLabel: "Move to Trash",
      });
    },
  })
);

$("#delete-permanent-btn").addEventListener("click", () =>
  runDelete({
    command: "delete_many_permanent",
    confirmFn: (sel) => {
      const totalBytes = sel.reduce((s, r) => s + r.size_bytes, 0);
      return askConfirm({
        title: "Permanently delete?",
        summary: `${sel.length} node_modules folders (${fmtSize(totalBytes)}) will be deleted.`,
        warning: "This bypasses the Trash. Files cannot be recovered.",
        requireType: true,
        okLabel: "Delete permanently",
        okDanger: true,
      });
    },
  })
);
