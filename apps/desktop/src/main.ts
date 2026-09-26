import { invoke } from "@tauri-apps/api/core";

type PlanDto = {
  method: string;
  capacity: number | null;
  jpegWarning: string | null;
  eofCaveat: string | null;
  coverPath: string;
  coverSize: number;
};

type HideResultDto = {
  outputPath: string;
  outputSize: number;
  extension: string;
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
};

let coverPath: string | null = null;
let payloadPath: string | null = null;
let stegoPath: string | null = null;
let keyfilePath: string | null = null;
let extractKeyfilePath: string | null = null;
let lastSavedExecutable: string | null = null;

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
}

function updateExtractEnabled() {
  const btn = $("btn-extract") as HTMLButtonElement | null;
  if (!btn) return;
  const pw = ($("extract-password") as HTMLInputElement | null)?.value ?? "";
  btn.disabled = !stegoPath || pw.length === 0;
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
  $("panel-hide")?.classList.toggle("hidden", tab !== "hide");
  $("panel-extract")?.classList.toggle("hidden", tab !== "extract");
  $("panel-demo")?.classList.toggle("hidden", tab !== "demo");
  $("panel-about")?.classList.toggle("hidden", tab !== "about");
  $("panel-lab")?.classList.toggle("hidden", tab !== "lab");
}

function hideRunBox() {
  lastSavedExecutable = null;
  $("run-box")?.classList.add("hidden");
}

async function onPickCover() {
  try {
    const plan = await invoke<PlanDto>("pick_cover");
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
    const caveat = $("plan-caveat")!;
    const text = plan.eofCaveat ?? plan.jpegWarning;
    if (text) {
      caveat.textContent = text;
      caveat.classList.remove("hidden");
    } else {
      caveat.classList.add("hidden");
    }
    setHideStatus("");
    updateHideEnabled();
  } catch (e) {
    if (String(e) !== "cancelled") setHideStatus(String(e), true);
  }
}

async function onPickPayload() {
  try {
    const [path, size, kind] = await invoke<[string, number, string]>(
      "pick_payload_file",
    );
    payloadPath = path;
    $("payload-path")!.textContent = `${path} (${formatBytes(size)})`;
    const hint = $("payload-kind-hint")!;
    if (kind === "executable") {
      hint.textContent =
        "Detected as executable/script — after extract you can Save, then optionally Run with confirm.";
      hint.classList.remove("hidden");
    } else {
      hint.classList.add("hidden");
    }
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
      lsbDepth: Number(($("lsb-depth") as HTMLSelectElement | null)?.value ?? "1"),
    });
    setHideStatus(
      `Saved ${result.outputPath} (${formatBytes(result.outputSize)}, .${result.extension})`,
    );
  } catch (e) {
    if (String(e) !== "cancelled") setHideStatus(String(e), true);
    else setHideStatus("");
  }
}

async function onPickStego() {
  try {
    const [path, size] = await invoke<[string, number]>("pick_stego_file");
    stegoPath = path;
    $("stego-path")!.textContent = `${path} (${formatBytes(size)})`;
    $("extract-text")?.classList.add("hidden");
    hideRunBox();
    setExtractStatus("");
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
      lsbDepth: 1,
    });
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
  $("payload-text")?.addEventListener("input", updateHideEnabled);
  $("btn-hide")?.addEventListener("click", onHide);
  $("btn-reveal-cover")?.addEventListener("click", async () => {
    if (!coverPath) return;
    try {
      await invoke("open_path", { path: coverPath });
    } catch (e) {
      setHideStatus(String(e), true);
    }
  });

  $("btn-stego")?.addEventListener("click", onPickStego);
  $("extract-password")?.addEventListener("input", updateExtractEnabled);
  $("btn-extract")?.addEventListener("click", onExtract);
  $("btn-run")?.addEventListener("click", onRun);
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
  $("btn-dismiss-wizard")?.addEventListener("click", () => {
    localStorage.setItem("openstego_wizard_done", "1");
    $("wizard-box")?.classList.add("hidden");
  });
  if (localStorage.getItem("openstego_wizard_done") === "1") {
    $("wizard-box")?.classList.add("hidden");
  }

  // Recent path history (no passwords).
  const histKey = "openstego_recent";
  const pushHist = (p: string) => {
    const arr: string[] = JSON.parse(localStorage.getItem(histKey) || "[]");
    const next = [p, ...arr.filter((x) => x !== p)].slice(0, 8);
    localStorage.setItem(histKey, JSON.stringify(next));
  };
  const origPickCover = onPickCover;
  // Wrap status updates to record history when paths change via existing handlers.
  const coverEl = $("cover-path");
  const obs = new MutationObserver(() => {
    const t = coverEl?.textContent ?? "";
    if (t && !t.startsWith("No cover")) pushHist(t.split(" (")[0]!);
  });
  if (coverEl) obs.observe(coverEl, { childList: true, characterData: true, subtree: true });

  document.querySelectorAll(".drop-zone").forEach((zone) => {
    zone.addEventListener("dragover", (e) => {
      e.preventDefault();
      zone.classList.add("dragover");
    });
    zone.addEventListener("dragleave", () => zone.classList.remove("dragover"));
    zone.addEventListener("drop", (e) => {
      e.preventDefault();
      zone.classList.remove("dragover");
      setHideStatus(
        "Drag-and-drop of OS paths needs the native file dialog in this build — use Choose cover / payload.",
      );
      void origPickCover;
    });
  });

  refreshModeUi();
});
