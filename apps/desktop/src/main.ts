import { invoke } from "@tauri-apps/api/core";
import { getCurrentWebview } from "@tauri-apps/api/webview";

type PlanDto = {
  method: string;
  capacity: number | null;
  jpegWarning: string | null;
  eofCaveat: string | null;
  capacityRisk: string | null;
  coverPath: string;
  coverSize: number;
  previewUrl: string | null;
};

type HideResultDto = {
  outputPath: string;
  outputSize: number;
  extension: string;
  previewUrl: string | null;
  diffUrl: string | null;
};

type ExtractResultDto = {
  isText: boolean;
  kind: string;
  filename: string | null;
  method: string;
  size: number;
  checksumHex: string;
  textPreview: string | null;
  savedPath: string | null;
  revealPreviewUrl: string | null;
};

type PayloadInfoDto = {
  path: string;
  size: number;
  kind: string;
};

type BatchResultDto = {
  items: { coverPath: string; ok: boolean; message: string; outputPath: string | null }[];
};

let coverPath: string | null = null;
let payloadPath: string | null = null;
let stegoPath: string | null = null;
let keyfilePath: string | null = null;
let extractKeyfilePath: string | null = null;
let lastSavedExecutable: string | null = null;
let batchCovers: string[] = [];
let batchPayloadPath: string | null = null;
let batchOutDir: string | null = null;
let labImagePath: string | null = null;
let activeDrop: "cover" | "payload" | "stego" | "batch-cover" | "lab" | null = null;

const HIST_KEY = "openstego_recent";
const DOC_SNIPPETS: Record<string, string> = {
  FORMAT:
    "FORMAT: OSMP metadata wrapper + crypto envelope versions. App semver is workspace version; breaking on-disk layouts need a new version byte.",
  CRYPTO:
    "CRYPTO: Argon2id (Fast/Balanced/Paranoid) → HKDF → AES-256-GCM. Optional keyfile mixes into the password material.",
  LEARNING:
    "LEARNING: Demo path is Hide → open cover normally → Extract → optional Run with confirm. No silent auto-run in OS viewers.",
  EMBEDDING:
    "EMBEDDING: Method A = keyed LSB in PNG/BMP (depth 1 or 2). Method B = keyed EOF append; PDF gets format-aware %%EOF handling.",
  THREAT_MODEL:
    "THREAT_MODEL: Educational privacy tool — not undetectable against a determined analyst. Passwords matter; LSB leaves statistical traces.",
};

function $(id: string) {
  return document.getElementById(id);
}

function setHideStatus(msg: string, isError = false) {
  const el = $("hide-status");
  if (!el) return;
  el.textContent = msg;
  el.classList.toggle("error", isError);
}

function setExtractStatus(msg: string, isError = false) {
  const el = $("extract-status");
  if (!el) return;
  el.textContent = msg;
  el.classList.toggle("error", isError);
}

function payloadMode(): "file" | "text" {
  const checked = document.querySelector(
    'input[name="payload-mode"]:checked',
  ) as HTMLInputElement | null;
  return checked?.value === "text" ? "text" : "file";
}

function refreshModeUi() {
  const mode = payloadMode();
  $("payload-file-box")?.classList.toggle("hidden", mode !== "file");
  $("payload-text-box")?.classList.toggle("hidden", mode !== "text");
  updateHideEnabled();
  void refreshPlan();
}

function updateHideEnabled() {
  const btn = $("btn-hide") as HTMLButtonElement | null;
  if (!btn) return;
  const pw = ($("password") as HTMLInputElement | null)?.value ?? "";
  const mode = payloadMode();
  const hasPayload =
    mode === "file"
      ? Boolean(payloadPath)
      : Boolean(($("payload-text") as HTMLTextAreaElement | null)?.value.trim());
  btn.disabled = !coverPath || !hasPayload || pw.length === 0;
  const reveal = $("btn-reveal-cover") as HTMLButtonElement | null;
  if (reveal) reveal.disabled = !coverPath;
  updateBatchEnabled();
}

function updateExtractEnabled() {
  const btn = $("btn-extract") as HTMLButtonElement | null;
  if (!btn) return;
  const pw = ($("extract-password") as HTMLInputElement | null)?.value ?? "";
  btn.disabled = !stegoPath || pw.length === 0;
  const reveal = $("btn-reveal-stego") as HTMLButtonElement | null;
  if (reveal) reveal.disabled = !stegoPath;
}

function updateBatchEnabled() {
  const btn = $("btn-batch-run") as HTMLButtonElement | null;
  if (!btn) return;
  const pw = ($("password") as HTMLInputElement | null)?.value ?? "";
  const hasPayload =
    Boolean(batchPayloadPath) ||
    (payloadMode() === "file" ? Boolean(payloadPath) : Boolean(($("payload-text") as HTMLTextAreaElement | null)?.value.trim()));
  btn.disabled = batchCovers.length === 0 || !batchOutDir || !hasPayload || pw.length === 0;
}

function formatBytes(n: number): string {
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KiB`;
  return `${(n / (1024 * 1024)).toFixed(2)} MiB`;
}

function switchTab(tab: string) {
  document.querySelectorAll(".tab").forEach((el) => {
    el.classList.toggle("active", (el as HTMLElement).dataset.tab === tab);
  });
  for (const id of ["hide", "extract", "batch", "demo", "lab", "about"]) {
    $(`panel-${id}`)?.classList.toggle("hidden", tab !== id);
  }
}

function hideRunBox() {
  lastSavedExecutable = null;
  $("run-box")?.classList.add("hidden");
}

function pushHist(p: string) {
  const arr: string[] = JSON.parse(localStorage.getItem(HIST_KEY) || "[]");
  const next = [p, ...arr.filter((x) => x !== p)].slice(0, 10);
  localStorage.setItem(HIST_KEY, JSON.stringify(next));
  renderRecent();
}

function renderRecent() {
  const list = $("recent-list");
  const panel = $("recent-panel");
  if (!list || !panel) return;
  const arr: string[] = JSON.parse(localStorage.getItem(HIST_KEY) || "[]");
  list.innerHTML = "";
  if (arr.length === 0) {
    panel.classList.add("hidden");
    return;
  }
  panel.classList.remove("hidden");
  for (const p of arr) {
    const li = document.createElement("li");
    const btn = document.createElement("button");
    btn.type = "button";
    btn.className = "linkish";
    btn.textContent = p;
    btn.title = p;
    btn.addEventListener("click", () => void applyRecentPath(p));
    li.appendChild(btn);
    list.appendChild(li);
  }
}

async function applyRecentPath(p: string) {
  const lower = p.toLowerCase();
  if (/\.(png|bmp|jpg|jpeg|gif|webp|pdf|zip|mp3)$/.test(lower)) {
    await applyCoverPath(p);
    switchTab("hide");
  } else {
    await applyPayloadPath(p);
    switchTab("hide");
  }
}

function showPlan(plan: PlanDto) {
  coverPath = plan.coverPath;
  $("cover-path")!.textContent = `${plan.coverPath} (${formatBytes(plan.coverSize)})`;
  $("plan-box")!.classList.remove("hidden");
  $("plan-method")!.textContent = plan.method;
  const capRow = $("plan-capacity-row")!;
  if (plan.capacity != null) {
    capRow.classList.remove("hidden");
    $("plan-capacity")!.textContent = formatBytes(plan.capacity);
  } else {
    capRow.classList.add("hidden");
  }
  const risk = $("plan-risk")!;
  if (plan.capacityRisk) {
    risk.textContent = plan.capacityRisk;
    risk.classList.remove("hidden");
  } else {
    risk.classList.add("hidden");
  }
  const caveat = $("plan-caveat")!;
  const text = plan.eofCaveat ?? plan.jpegWarning;
  if (text) {
    caveat.textContent = text;
    caveat.classList.remove("hidden");
  } else {
    caveat.classList.add("hidden");
  }
  const box = $("cover-preview-box")!;
  const img = $("cover-preview") as HTMLImageElement;
  if (plan.previewUrl) {
    img.src = plan.previewUrl;
    box.classList.remove("hidden");
  } else {
    box.classList.add("hidden");
  }
  $("stego-preview-fig")?.classList.add("hidden");
  $("diff-preview-fig")?.classList.add("hidden");
  pushHist(plan.coverPath);
  updateHideEnabled();
}

async function refreshPlan() {
  if (!coverPath) return;
  try {
    let payloadLen: number | undefined;
    if (payloadMode() === "file" && payloadPath) {
      // size unknown cheaply; plan without exact size still works
    } else if (payloadMode() === "text") {
      payloadLen = ($("payload-text") as HTMLTextAreaElement).value.length;
    }
    const plan = await invoke<PlanDto>("inspect_cover_path", {
      path: coverPath,
      payloadLen: payloadLen ?? null,
      lsbDepth: Number(($("lsb-depth") as HTMLSelectElement).value),
      adaptiveLsb: ($("adaptive-lsb") as HTMLInputElement).checked,
    });
    showPlan(plan);
  } catch {
    /* keep previous plan UI */
  }
}

async function applyCoverPath(path: string) {
  try {
    const plan = await invoke<PlanDto>("inspect_cover_path", {
      path,
      payloadLen: null,
      lsbDepth: Number(($("lsb-depth") as HTMLSelectElement | null)?.value ?? "1"),
      adaptiveLsb: ($("adaptive-lsb") as HTMLInputElement | null)?.checked ?? false,
    });
    showPlan(plan);
    setHideStatus("");
  } catch (e) {
    setHideStatus(String(e), true);
  }
}

async function applyPayloadPath(path: string) {
  try {
    const info = await invoke<PayloadInfoDto>("set_payload_path", { path });
    payloadPath = info.path;
    $("payload-path")!.textContent = `${info.path} (${formatBytes(info.size)})`;
    const hint = $("payload-kind-hint")!;
    if (info.kind === "executable") {
      hint.textContent =
        "Detected as executable/script — after extract you can Save, then optionally Run with confirm.";
      hint.classList.remove("hidden");
    } else {
      hint.classList.add("hidden");
    }
    pushHist(info.path);
    updateHideEnabled();
    void refreshPlan();
  } catch (e) {
    setHideStatus(String(e), true);
  }
}

async function applyStegoPath(path: string) {
  try {
    const [p, size, preview] = await invoke<[string, number, string | null]>(
      "set_stego_path",
      { path },
    );
    stegoPath = p;
    $("stego-path")!.textContent = `${p} (${formatBytes(size)})`;
    const box = $("reveal-box")!;
    const img = $("reveal-preview") as HTMLImageElement;
    if (preview) {
      img.src = preview;
      box.classList.remove("hidden");
    } else {
      box.classList.add("hidden");
    }
    $("extract-text")?.classList.add("hidden");
    hideRunBox();
    setExtractStatus("");
    pushHist(p);
    updateExtractEnabled();
  } catch (e) {
    setExtractStatus(String(e), true);
  }
}

async function onPickCover() {
  try {
    const plan = await invoke<PlanDto>("pick_cover");
    showPlan(plan);
    setHideStatus("");
  } catch (e) {
    if (String(e) !== "cancelled") setHideStatus(String(e), true);
  }
}

async function onPickPayload() {
  try {
    const info = await invoke<PayloadInfoDto>("pick_payload_file");
    payloadPath = info.path;
    $("payload-path")!.textContent = `${info.path} (${formatBytes(info.size)})`;
    const hint = $("payload-kind-hint")!;
    if (info.kind === "executable") {
      hint.textContent =
        "Detected as executable/script — after extract you can Save, then optionally Run with confirm.";
      hint.classList.remove("hidden");
    } else {
      hint.classList.add("hidden");
    }
    pushHist(info.path);
    updateHideEnabled();
  } catch (e) {
    if (String(e) !== "cancelled") setHideStatus(String(e), true);
  }
}

async function onPasswordInput() {
  const pw = ($("password") as HTMLInputElement).value;
  const label = await invoke<string>("password_strength", { password: pw });
  const el = $("pw-strength")!;
  el.textContent = `Strength: ${label}`;
  el.dataset.level = label;
  updateHideEnabled();
}

async function onHide() {
  if (!coverPath) return;
  setHideStatus("Working…");
  const mode = payloadMode();
  try {
    const result = await invoke<HideResultDto>("hide_payload", {
      coverPath,
      payloadPath: mode === "file" ? payloadPath : null,
      textPayload:
        mode === "text"
          ? ($("payload-text") as HTMLTextAreaElement).value
          : null,
      password: ($("password") as HTMLInputElement).value,
      verify: ($("verify-roundtrip") as HTMLInputElement).checked,
      profile: ($("kdf-profile") as HTMLSelectElement).value,
      keyfilePath,
      adaptiveLsb: ($("adaptive-lsb") as HTMLInputElement).checked,
      lsbDepth: Number(($("lsb-depth") as HTMLSelectElement).value),
      outputPath: null,
    });
    setHideStatus(
      `Saved ${result.outputPath} (${formatBytes(result.outputSize)}, .${result.extension})`,
    );
    pushHist(result.outputPath);
    const box = $("cover-preview-box")!;
    if (result.previewUrl) {
      ($("stego-preview") as HTMLImageElement).src = result.previewUrl;
      $("stego-preview-fig")!.classList.remove("hidden");
      box.classList.remove("hidden");
    }
    if (result.diffUrl) {
      ($("diff-preview") as HTMLImageElement).src = result.diffUrl;
      $("diff-preview-fig")!.classList.remove("hidden");
      box.classList.remove("hidden");
    }
  } catch (e) {
    if (String(e) !== "cancelled") setHideStatus(String(e), true);
    else setHideStatus("");
  }
}

async function onPickStego() {
  try {
    const [path, size, preview] = await invoke<[string, number, string | null]>(
      "pick_stego_file",
    );
    stegoPath = path;
    $("stego-path")!.textContent = `${path} (${formatBytes(size)})`;
    const box = $("reveal-box")!;
    if (preview) {
      ($("reveal-preview") as HTMLImageElement).src = preview;
      box.classList.remove("hidden");
    } else {
      box.classList.add("hidden");
    }
    $("extract-text")?.classList.add("hidden");
    hideRunBox();
    setExtractStatus("");
    pushHist(path);
    updateExtractEnabled();
  } catch (e) {
    if (String(e) !== "cancelled") setExtractStatus(String(e), true);
  }
}

async function onExtract() {
  if (!stegoPath) return;
  setExtractStatus("Working…");
  $("extract-text")?.classList.add("hidden");
  hideRunBox();
  try {
    const result = await invoke<ExtractResultDto>("extract_payload", {
      stegoPath,
      password: ($("extract-password") as HTMLInputElement).value,
      keyfilePath: extractKeyfilePath,
      adaptiveLsb: ($("extract-adaptive-lsb") as HTMLInputElement).checked,
      lsbDepth: Number(($("extract-lsb-depth") as HTMLSelectElement).value),
    });
    if (result.revealPreviewUrl) {
      ($("reveal-preview") as HTMLImageElement).src = result.revealPreviewUrl;
      $("reveal-box")!.classList.remove("hidden");
    }
    if (result.isText && result.textPreview != null) {
      const pre = $("extract-text")!;
      pre.textContent = result.textPreview;
      pre.classList.remove("hidden");
      setExtractStatus(
        `Recovered text via ${result.method} (${formatBytes(result.size)})`,
      );
    } else {
      setExtractStatus(
        `Saved ${result.savedPath} via ${result.method} [${result.kind}] (${formatBytes(result.size)}${
          result.filename ? `, as ${result.filename}` : ""
        })`,
      );
      if (result.kind === "executable" && result.savedPath) {
        lastSavedExecutable = result.savedPath;
        const shortHash = result.checksumHex.slice(0, 12);
        $("run-meta")!.textContent =
          `${result.filename ?? "file"} · ${formatBytes(result.size)} · sha256 ${shortHash}…`;
        $("run-box")!.classList.remove("hidden");
      }
    }
  } catch (e) {
    if (String(e) !== "cancelled") setExtractStatus(String(e), true);
    else setExtractStatus("");
  }
}

async function onRun() {
  if (!lastSavedExecutable) return;
  const ok = window.confirm(
    "You extracted this file yourself from a cover inside Open Stego.\n\nRun it now? Only continue if you trust the source (educational demo).",
  );
  if (!ok) return;
  const ok2 = window.confirm(
    "Final confirm: start the recovered program/script?",
  );
  if (!ok2) return;
  try {
    await invoke("run_extracted", { path: lastSavedExecutable });
    setExtractStatus(`Started: ${lastSavedExecutable}`);
  } catch (e) {
    setExtractStatus(String(e), true);
  }
}

async function onPickKeyfile(forExtract: boolean) {
  try {
    const [path, size] = await invoke<[string, number]>("pick_keyfile");
    if (forExtract) {
      extractKeyfilePath = path;
      $("extract-keyfile-path")!.textContent = `${path} (${formatBytes(size)})`;
    } else {
      keyfilePath = path;
      $("keyfile-path")!.textContent = `${path} (${formatBytes(size)})`;
    }
  } catch (e) {
    if (String(e) !== "cancelled") {
      if (forExtract) setExtractStatus(String(e), true);
      else setHideStatus(String(e), true);
    }
  }
}

function renderBatchList() {
  const ul = $("batch-list");
  if (!ul) return;
  ul.innerHTML = "";
  for (const p of batchCovers) {
    const li = document.createElement("li");
    li.textContent = p;
    ul.appendChild(li);
  }
  updateBatchEnabled();
}

async function handleDroppedPaths(paths: string[]) {
  if (!paths.length) return;
  const kind = activeDrop;
  activeDrop = null;
  if (kind === "cover" || (!kind && paths.length === 1)) {
    await applyCoverPath(paths[0]!);
    return;
  }
  if (kind === "payload") {
    await applyPayloadPath(paths[0]!);
    return;
  }
  if (kind === "stego") {
    await applyStegoPath(paths[0]!);
    return;
  }
  if (kind === "lab") {
    labImagePath = paths[0]!;
    $("lab-path")!.textContent = labImagePath;
    ($("btn-lab-live") as HTMLButtonElement).disabled = false;
    pushHist(labImagePath);
    return;
  }
  if (kind === "batch-cover") {
    for (const p of paths) {
      if (!batchCovers.includes(p)) batchCovers.push(p);
    }
    renderBatchList();
    return;
  }
  // Default: first path as cover
  await applyCoverPath(paths[0]!);
}

window.addEventListener("DOMContentLoaded", () => {
  invoke<string>("app_version").then((v) => {
    $("version-msg")!.textContent = `stego-core ${v}`;
  });

  document.querySelectorAll(".tab").forEach((el) => {
    el.addEventListener("click", () => {
      const tab = (el as HTMLElement).dataset.tab;
      if (tab) switchTab(tab);
    });
  });

  document.querySelectorAll('input[name="payload-mode"]').forEach((el) => {
    el.addEventListener("change", refreshModeUi);
  });
  $("btn-cover")?.addEventListener("click", onPickCover);
  $("btn-payload")?.addEventListener("click", onPickPayload);
  $("btn-keyfile")?.addEventListener("click", () => onPickKeyfile(false));
  $("btn-extract-keyfile")?.addEventListener("click", () => onPickKeyfile(true));
  $("password")?.addEventListener("input", onPasswordInput);
  $("payload-text")?.addEventListener("input", () => {
    updateHideEnabled();
    void refreshPlan();
  });
  $("lsb-depth")?.addEventListener("change", () => void refreshPlan());
  $("adaptive-lsb")?.addEventListener("change", () => void refreshPlan());
  $("btn-hide")?.addEventListener("click", onHide);
  $("btn-reveal-cover")?.addEventListener("click", async () => {
    if (!coverPath) return;
    try {
      await invoke("open_path", { path: coverPath });
    } catch (e) {
      setHideStatus(String(e), true);
    }
  });
  $("btn-reveal-stego")?.addEventListener("click", async () => {
    if (!stegoPath) return;
    try {
      await invoke("open_path", { path: stegoPath });
    } catch (e) {
      setExtractStatus(String(e), true);
    }
  });

  $("btn-stego")?.addEventListener("click", onPickStego);
  $("extract-password")?.addEventListener("input", updateExtractEnabled);
  $("btn-extract")?.addEventListener("click", onExtract);
  $("btn-run")?.addEventListener("click", onRun);

  $("btn-batch-add")?.addEventListener("click", async () => {
    try {
      // reuse cover picker repeatedly is awkward; ask user to drop or pick one-by-one via cover dialog
      const plan = await invoke<PlanDto>("pick_cover");
      if (!batchCovers.includes(plan.coverPath)) batchCovers.push(plan.coverPath);
      renderBatchList();
      pushHist(plan.coverPath);
    } catch (e) {
      if (String(e) !== "cancelled") setHideStatus(String(e), true);
    }
  });
  $("btn-batch-payload")?.addEventListener("click", async () => {
    try {
      const info = await invoke<PayloadInfoDto>("pick_payload_file");
      batchPayloadPath = info.path;
      $("batch-payload-path")!.textContent = info.path;
      updateBatchEnabled();
    } catch (e) {
      if (String(e) !== "cancelled") setHideStatus(String(e), true);
    }
  });
  $("btn-batch-outdir")?.addEventListener("click", async () => {
    try {
      batchOutDir = await invoke<string>("pick_output_dir");
      $("batch-outdir")!.textContent = batchOutDir;
      updateBatchEnabled();
    } catch (e) {
      if (String(e) !== "cancelled") setHideStatus(String(e), true);
    }
  });
  $("btn-batch-run")?.addEventListener("click", async () => {
    if (!batchOutDir) return;
    const mode = payloadMode();
    const out = $("batch-out")!;
    out.classList.remove("hidden");
    out.textContent = "Working…";
    try {
      const result = await invoke<BatchResultDto>("batch_hide", {
        coverPaths: batchCovers,
        payloadPath: batchPayloadPath ?? (mode === "file" ? payloadPath : null),
        textPayload:
          !batchPayloadPath && mode === "text"
            ? ($("payload-text") as HTMLTextAreaElement).value
            : null,
        password: ($("password") as HTMLInputElement).value,
        profile: ($("kdf-profile") as HTMLSelectElement).value,
        keyfilePath,
        adaptiveLsb: ($("adaptive-lsb") as HTMLInputElement).checked,
        lsbDepth: Number(($("lsb-depth") as HTMLSelectElement).value),
        outputDir: batchOutDir,
      });
      out.textContent = result.items
        .map(
          (i) =>
            `${i.ok ? "OK" : "FAIL"}  ${i.coverPath}\n  ${i.message}${i.outputPath ? `\n  → ${i.outputPath}` : ""}`,
        )
        .join("\n\n");
    } catch (e) {
      out.textContent = String(e);
    }
  });

  $("btn-lab-run")?.addEventListener("click", async () => {
    try {
      const text = await invoke<string>("lab_lsb_demo");
      const pre = $("lab-out")!;
      pre.textContent = text;
      pre.classList.remove("hidden");
    } catch (e) {
      setExtractStatus(String(e), true);
    }
  });
  $("btn-lab-pick")?.addEventListener("click", async () => {
    try {
      labImagePath = await invoke<string>("pick_lab_image");
      $("lab-path")!.textContent = labImagePath;
      ($("btn-lab-live") as HTMLButtonElement).disabled = false;
      pushHist(labImagePath);
    } catch (e) {
      if (String(e) !== "cancelled") setExtractStatus(String(e), true);
    }
  });
  $("btn-lab-live")?.addEventListener("click", async () => {
    if (!labImagePath) return;
    try {
      const text = await invoke<string>("lab_lsb_on_image", { path: labImagePath });
      const pre = $("lab-out")!;
      pre.textContent = text;
      pre.classList.remove("hidden");
    } catch (e) {
      setExtractStatus(String(e), true);
    }
  });

  $("btn-dismiss-wizard")?.addEventListener("click", () => {
    localStorage.setItem("openstego_wizard_done", "1");
    $("wizard-box")?.classList.add("hidden");
  });
  if (localStorage.getItem("openstego_wizard_done") === "1") {
    $("wizard-box")?.classList.add("hidden");
  }

  document.querySelectorAll("[data-doc]").forEach((el) => {
    el.addEventListener("click", (ev) => {
      ev.preventDefault();
      const key = (el as HTMLElement).dataset.doc ?? "";
      $("doc-snippet")!.textContent = DOC_SNIPPETS[key] ?? "";
    });
  });

  document.querySelectorAll(".drop-zone").forEach((zone) => {
    zone.addEventListener("dragenter", () => {
      activeDrop = ((zone as HTMLElement).dataset.drop as typeof activeDrop) ?? null;
      zone.classList.add("dragover");
    });
    zone.addEventListener("dragover", (e) => {
      e.preventDefault();
      zone.classList.add("dragover");
    });
    zone.addEventListener("dragleave", () => zone.classList.remove("dragover"));
    zone.addEventListener("drop", (e) => {
      e.preventDefault();
      zone.classList.remove("dragover");
    });
  });

  void getCurrentWebview()
    .onDragDropEvent((event) => {
      if (event.payload.type === "over") {
        document.querySelectorAll(".drop-zone").forEach((z) => z.classList.add("dragover"));
      } else if (event.payload.type === "leave") {
        document.querySelectorAll(".drop-zone").forEach((z) => z.classList.remove("dragover"));
      } else if (event.payload.type === "drop") {
        document.querySelectorAll(".drop-zone").forEach((z) => z.classList.remove("dragover"));
        void handleDroppedPaths(event.payload.paths);
      }
    })
    .catch(() => {
      /* webview API unavailable outside Tauri */
    });

  renderRecent();
  refreshModeUi();
});
