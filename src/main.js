import { invoke } from "https://esm.sh/@tauri-apps/api@2/core";

const $ = (sel) => document.querySelector(sel);

$("#choose-btn").addEventListener("click", async () => {
  const root = await invoke("pick_root_folder");
  if (!root) return;
  startScan(root);
});

async function startScan(root) {
  $("#empty-state").hidden = true;
  $("#status").textContent = `Scanning ${root}…`;
  // Wired up further in Task 10
  await invoke("scan", { root });
}
