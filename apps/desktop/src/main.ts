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
  filename: string | null;
  method: string;
  size: number;
  textPreview: string | null;
  savedPath: string | null;
};

let coverPath: string | null = null;
let payloadPath: string | null = null;
let stegoPath: string | null = null;

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
    const [path, size] = await invoke<[string, number]>("pick_payload_file");
    payloadPath = path;
    $("payload-path")!.textContent = `${path} (${formatBytes(size)})`;
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
  try {
    const result = await invoke<ExtractResultDto>("extract_payload", {
      stegoPath,
      password: ($("extract-password") as HTMLInputElement).value,
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
        `Saved ${result.savedPath} via ${result.method} (${formatBytes(result.size)}${
          result.filename ? `, as ${result.filename}` : ""
        })`,
      );
    }
  } catch (e) {
    if (String(e) !== "cancelled") setExtractStatus(String(e), true);
    else setExtractStatus("");
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
  $("password")?.addEventListener("input", onPasswordInput);
  $("payload-text")?.addEventListener("input", updateHideEnabled);
  $("btn-hide")?.addEventListener("click", onHide);

  $("btn-stego")?.addEventListener("click", onPickStego);
  $("extract-password")?.addEventListener("input", updateExtractEnabled);
  $("btn-extract")?.addEventListener("click", onExtract);

  refreshModeUi();
});
