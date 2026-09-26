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

let coverPath: string | null = null;
let payloadPath: string | null = null;

function $(id: string) {
  return document.getElementById(id);
}

function setStatus(msg: string, isError = false) {
  const el = $("hide-status");
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

function formatBytes(n: number): string {
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KiB`;
  return `${(n / (1024 * 1024)).toFixed(2)} MiB`;
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
    setStatus("");
    updateHideEnabled();
  } catch (e) {
    if (String(e) !== "cancelled") setStatus(String(e), true);
  }
}

async function onPickPayload() {
  try {
    const [path, size] = await invoke<[string, number]>("pick_payload_file");
    payloadPath = path;
    $("payload-path")!.textContent = `${path} (${formatBytes(size)})`;
    updateHideEnabled();
  } catch (e) {
    if (String(e) !== "cancelled") setStatus(String(e), true);
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
  setStatus("Working…");
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
    setStatus(
      `Saved ${result.outputPath} (${formatBytes(result.outputSize)}, .${result.extension})`,
    );
  } catch (e) {
    if (String(e) !== "cancelled") setStatus(String(e), true);
    else setStatus("");
  }
}

window.addEventListener("DOMContentLoaded", () => {
  invoke<string>("app_version").then((v) => {
    $("version-msg")!.textContent = `stego-core ${v}`;
  });

  document.querySelectorAll('input[name="payload-mode"]').forEach((el) => {
    el.addEventListener("change", refreshModeUi);
  });
  $("btn-cover")?.addEventListener("click", onPickCover);
  $("btn-payload")?.addEventListener("click", onPickPayload);
  $("password")?.addEventListener("input", onPasswordInput);
  $("payload-text")?.addEventListener("input", updateHideEnabled);
  $("btn-hide")?.addEventListener("click", onHide);
  refreshModeUi();
});
