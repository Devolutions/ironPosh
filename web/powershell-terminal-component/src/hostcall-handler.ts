import type { Terminal } from "@xterm/xterm";
import type {
  JsCoordinates,
  JsHostCall,
  JsKeyInfo,
  JsPSCredential,
  JsPsValue,
  JsRectangle,
  JsSize,
  TypedHostCallHandler,
} from "../../../crates/ironposh-web/pkg/ironposh_web";

export interface HostCallHandlerConfig {
  terminal: Terminal;
  hostName?: string;
  hostVersion?: string;
  instanceId?: string;
  culture?: string;
  uiCulture?: string;
  beginHostCallInput?: () => void;
  endHostCallInput?: () => void;
}

/**
 * Creates a PowerShell host call handler that integrates with xterm.js terminal
 * This follows the "let it crash" philosophy - only implement what we know how to handle
 */
export function createHostCallHandler(config: HostCallHandlerConfig) {
  const {
    terminal,
    hostName = "IronPoshHost",
    hostVersion = "1.0.0",
    instanceId = crypto.randomUUID(),
    culture = "en-US",
    uiCulture = "en-US",
    beginHostCallInput,
    endHostCallInput,
  } = config;

  const withHostCallInput = async <T>(fn: () => Promise<T>): Promise<T> => {
    beginHostCallInput?.();
    try {
      return await fn();
    } finally {
      endHostCallInput?.();
    }
  };

  const state = {
    cursorPosition: { x: 0, y: 0 } satisfies JsCoordinates,
    windowPosition: { x: 0, y: 0 } satisfies JsCoordinates,
    foregroundColor: 7,
    backgroundColor: 0,
    cursorSize: 25,
    windowTitle: "PowerShell",
    runspaceStack: [] as JsPsValue[],
  };

  const keyQueue: JsKeyInfo[] = [];
  const keyWaiters: Array<(k: JsKeyInfo) => void> = [];

  const enqueueKey = (keyInfo: JsKeyInfo) => {
    const waiter = keyWaiters.shift();
    if (waiter) {
      waiter(keyInfo);
      return;
    }
    keyQueue.push(keyInfo);
  };

  const normalizeChar = (s: string): string => {
    if (s.length === 0) return "\u0000";
    if (s.length === 1) return s;
    return "\u0000";
  };

  const controlKeyStateFromEvent = (ev: KeyboardEvent): number => {
    // Best-effort mapping to Windows CONTROL_KEY_STATE:
    // LEFT_ALT_PRESSED=0x2, LEFT_CTRL_PRESSED=0x8, SHIFT_PRESSED=0x10.
    let v = 0;
    if (ev.altKey) v |= 0x2;
    if (ev.ctrlKey) v |= 0x8;
    if (ev.shiftKey) v |= 0x10;
    return v;
  };

  // xterm v5 provides onKey; fallback to onData if unavailable.
  // We attach once per handler instance.
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const anyTerminal = terminal as any;
  if (typeof anyTerminal.onKey === "function") {
    anyTerminal.onKey((e: { key: string; domEvent: KeyboardEvent }) => {
      const key = e.key;
      const domEvent = e.domEvent;
      const character =
        key === "\r" ? "\r" : key === "\n" ? "\n" : normalizeChar(key);
      enqueueKey({
        virtualKeyCode:
          // eslint-disable-next-line deprecation/deprecation
          (domEvent as any).keyCode ?? character.charCodeAt(0) ?? 0,
        character,
        controlKeyState: controlKeyStateFromEvent(domEvent),
        keyDown: true,
      });
    });
  } else if (typeof anyTerminal.onData === "function") {
    anyTerminal.onData((data: string) => {
      for (const ch of data) {
        enqueueKey({
          virtualKeyCode: ch.charCodeAt(0),
          character: normalizeChar(ch),
          controlKeyState: 0,
          keyDown: true,
        });
      }
    });
  }

  const sgrColor = (c: number, isBackground: boolean): number => {
    // Treat incoming values as PowerShell ConsoleColor (0-15).
    const idx = c & 0xf;
    const base = isBackground ? 40 : 30;
    const brightBase = isBackground ? 100 : 90;
    const isBright = idx >= 8;
    const mapped = idx & 0x7;
    const table = [0, 4, 2, 6, 1, 5, 3, 7]; // black, blue, green, cyan, red, magenta, yellow, white
    const code = table[mapped];
    return (isBright ? brightBase : base) + code;
  };

  const consoleColorBaseFromAnsi = (ansiBase: number): number => {
    // ANSI base index: 0=black,1=red,2=green,3=yellow,4=blue,5=magenta,6=cyan,7=white
    const table = [0, 4, 2, 6, 1, 5, 3, 7];
    return table[ansiBase & 0x7] ?? 7;
  };

  const xtermPaletteIndexToRgb = (idx: number): { r: number; g: number; b: number } => {
    if (idx < 16) {
      const standard: Array<[number, number, number]> = [
        [0, 0, 0],
        [205, 0, 0],
        [0, 205, 0],
        [205, 205, 0],
        [0, 0, 238],
        [205, 0, 205],
        [0, 205, 205],
        [229, 229, 229],
        [127, 127, 127],
        [255, 0, 0],
        [0, 255, 0],
        [255, 255, 0],
        [92, 92, 255],
        [255, 0, 255],
        [0, 255, 255],
        [255, 255, 255],
      ];
      const rgb = standard[idx] ?? [255, 255, 255];
      return { r: rgb[0], g: rgb[1], b: rgb[2] };
    }

    if (idx >= 16 && idx <= 231) {
      const n = idx - 16;
      const r = Math.floor(n / 36);
      const g = Math.floor((n % 36) / 6);
      const b = n % 6;
      const steps = [0, 95, 135, 175, 215, 255];
      return { r: steps[r] ?? 0, g: steps[g] ?? 0, b: steps[b] ?? 0 };
    }

    if (idx >= 232 && idx <= 255) {
      const level = 8 + (idx - 232) * 10;
      return { r: level, g: level, b: level };
    }

    return { r: 255, g: 255, b: 255 };
  };

  const nearestConsoleColor = (rgb: { r: number; g: number; b: number }): number => {
    const palette: Array<{ idx: number; r: number; g: number; b: number }> = [
      { idx: 0, r: 0, g: 0, b: 0 }, // Black
      { idx: 1, r: 0, g: 0, b: 128 }, // DarkBlue
      { idx: 2, r: 0, g: 128, b: 0 }, // DarkGreen
      { idx: 3, r: 0, g: 128, b: 128 }, // DarkCyan
      { idx: 4, r: 128, g: 0, b: 0 }, // DarkRed
      { idx: 5, r: 128, g: 0, b: 128 }, // DarkMagenta
      { idx: 6, r: 128, g: 128, b: 0 }, // DarkYellow
      { idx: 7, r: 192, g: 192, b: 192 }, // Gray
      { idx: 8, r: 128, g: 128, b: 128 }, // DarkGray
      { idx: 9, r: 0, g: 0, b: 255 }, // Blue
      { idx: 10, r: 0, g: 255, b: 0 }, // Green
      { idx: 11, r: 0, g: 255, b: 255 }, // Cyan
      { idx: 12, r: 255, g: 0, b: 0 }, // Red
      { idx: 13, r: 255, g: 0, b: 255 }, // Magenta
      { idx: 14, r: 255, g: 255, b: 0 }, // Yellow
      { idx: 15, r: 255, g: 255, b: 255 }, // White
    ];

    let best = 7;
    let bestDist = Number.POSITIVE_INFINITY;
    for (const p of palette) {
      const dr = rgb.r - p.r;
      const dg = rgb.g - p.g;
      const db = rgb.b - p.b;
      const dist = dr * dr + dg * dg + db * db;
      if (dist < bestDist) {
        bestDist = dist;
        best = p.idx;
      }
    }
    return best;
  };

  const consoleColorFromXtermCellFg = (
    cell: ReturnType<Terminal["buffer"]["active"]["getNullCell"]>,
  ): number => {
    if (cell.isFgDefault()) return state.foregroundColor;
    if (cell.isFgPalette()) {
      const idx = cell.getFgColor();
      if (idx >= 0 && idx <= 15) {
        const base = consoleColorBaseFromAnsi(idx & 0x7);
        return (idx >= 8 ? 8 : 0) + base;
      }
      return nearestConsoleColor(xtermPaletteIndexToRgb(idx));
    }
    if (cell.isFgRGB()) {
      const rgb = cell.getFgColor();
      return nearestConsoleColor({
        r: (rgb >> 16) & 0xff,
        g: (rgb >> 8) & 0xff,
        b: rgb & 0xff,
      });
    }
    return state.foregroundColor;
  };

  const consoleColorFromXtermCellBg = (
    cell: ReturnType<Terminal["buffer"]["active"]["getNullCell"]>,
  ): number => {
    if (cell.isBgDefault()) return state.backgroundColor;
    if (cell.isBgPalette()) {
      const idx = cell.getBgColor();
      if (idx >= 0 && idx <= 15) {
        const base = consoleColorBaseFromAnsi(idx & 0x7);
        return (idx >= 8 ? 8 : 0) + base;
      }
      return nearestConsoleColor(xtermPaletteIndexToRgb(idx));
    }
    if (cell.isBgRGB()) {
      const rgb = cell.getBgColor();
      return nearestConsoleColor({
        r: (rgb >> 16) & 0xff,
        g: (rgb >> 8) & 0xff,
        b: rgb & 0xff,
      });
    }
    return state.backgroundColor;
  };

  const withColor = (fg: number, bg: number, text: string) => {
    const seq = `\x1b[${sgrColor(fg, false)};${sgrColor(bg, true)}m`;
    const reset = `\x1b[${sgrColor(state.foregroundColor, false)};${sgrColor(
      state.backgroundColor,
      true,
    )}m`;
    terminal.write(seq + text + reset);
  };

  const cursorTo = (x: number, y: number) => {
    const xx = Math.max(0, x);
    const yy = Math.max(0, y);
    state.cursorPosition = { x: xx, y: yy };
    terminal.write(`\x1b[${yy + 1};${xx + 1}H`);
  };

  const applyCurrentColors = () => {
    terminal.write(
      `\x1b[${sgrColor(state.foregroundColor, false)};${sgrColor(state.backgroundColor, true)}m`,
    );
  };

  const rectCoversViewport = (rect: JsRectangle) =>
    rect.left <= 0 &&
    rect.top <= 0 &&
    rect.right >= terminal.cols - 1 &&
    rect.bottom >= terminal.rows - 1;

  const jsPsValueStr = (s: string): JsPsValue => ({
    kind: "primitive",
    value: { kind: "str", value: s },
  });

  const utf16leBytes = (s: string): number[] => {
    const out: number[] = [];
    for (let i = 0; i < s.length; i++) {
      const code = s.charCodeAt(i);
      out.push(code & 0xff, (code >> 8) & 0xff);
    }
    return out;
  };

  const readKeyAsync = async (): Promise<JsKeyInfo> => {
    const next = keyQueue.shift();
    if (next) return next;
    return await new Promise<JsKeyInfo>((resolve) => {
      keyWaiters.push(resolve);
    });
  };

  const readKeyOptions = (raw: number) => {
    // System.Management.Automation.Host.ReadKeyOptions (bitflags)
    // IncludeKeyDown = 1, NoEcho = 2, AllowCtrlC = 4
    return {
      includeKeyDown: (raw & 0x1) !== 0,
      noEcho: (raw & 0x2) !== 0,
      allowCtrlC: (raw & 0x4) !== 0,
    };
  };

  const readLineFromTerminal = async (opts: { echo: boolean }): Promise<string> => {
    let buf = "";

    while (true) {
      const k = await readKeyAsync();
      const ch = k.character;

      if (ch === "\r" || ch === "\n") {
        if (opts.echo) terminal.writeln("");
        return buf;
      }

      // Ctrl+C
      if (ch === "\u0003") {
        throw new Error("ReadLine cancelled (Ctrl+C)");
      }

      // Backspace (xterm commonly sends DEL)
      if (ch === "\b" || ch === "\u007f") {
        if (buf.length > 0) {
          buf = buf.slice(0, -1);
          if (opts.echo) terminal.write("\b \b");
        }
        continue;
      }

      if (ch === "\u0000") continue;

      // Ignore other control chars
      if (ch.charCodeAt(0) < 0x20) continue;

      buf += ch;
      if (opts.echo) terminal.write(ch);
    }
  };

  const promptCredentialFromTerminal = async (opts: {
    caption: string;
    message: string;
    defaultUserName: string;
  }): Promise<JsPSCredential> => {
    terminal.writeln(opts.caption);
    if (opts.message) terminal.writeln(opts.message);

    terminal.write("Username: ");
    const userNameRaw = await readLineFromTerminal({ echo: true });
    const userName = userNameRaw.trim().length > 0 ? userNameRaw : opts.defaultUserName;

    terminal.write("Password: ");
    const password = await readLineFromTerminal({ echo: false });
    terminal.writeln("");

    return { userName, password };
  };

  const promptToJsPsValue = (
    field: {
      name: string;
      label: string;
      helpMessage: string;
      isMandatory: boolean;
      parameterType: string;
      defaultValueDebug?: string;
    },
    raw: string,
  ): JsPsValue => {
    const ty = (field.parameterType ?? "").toLowerCase();

    if (ty.includes("securestring")) {
      return {
        kind: "primitive",
        value: { kind: "secureString", value: utf16leBytes(raw) },
      };
    }

    if (ty.includes("bool") || ty.includes("boolean")) {
      const v = raw.trim().toLowerCase();
      if (v === "true" || v === "1" || v === "yes" || v === "y") {
        return { kind: "primitive", value: { kind: "bool", value: true } };
      }
      if (v === "false" || v === "0" || v === "no" || v === "n") {
        return { kind: "primitive", value: { kind: "bool", value: false } };
      }
      throw new Error(
        `Prompt field '${field.name}' expects boolean but got '${raw}'`,
      );
    }

    if (ty.includes("int32") || ty.includes("system.int32")) {
      const n = Number.parseInt(raw, 10);
      if (!Number.isFinite(n)) {
        throw new Error(
          `Prompt field '${field.name}' expects int32 but got '${raw}'`,
        );
      }
      return { kind: "primitive", value: { kind: "i32", value: n } };
    }

    if (ty.includes("int64") || ty.includes("system.int64")) {
      const n = Number.parseInt(raw, 10);
      if (!Number.isFinite(n)) {
        throw new Error(
          `Prompt field '${field.name}' expects int64 but got '${raw}'`,
        );
      }
      return { kind: "primitive", value: { kind: "i64", value: n } };
    }

    return jsPsValueStr(raw);
  };

  const isSecureStringType = (parameterType: string | undefined): boolean =>
    (parameterType ?? "").toLowerCase().includes("securestring");

  const handlerImpl = async (hostCall: JsHostCall): Promise<unknown> => {
    // ===== IMPLEMENTED HOST CALLS (matching ironposh-client-tokio/src/hostcall.rs) =====

    // Basic host information
    if ("GetName" in hostCall) {
      return hostName;
    }

    if ("GetVersion" in hostCall) {
      return hostVersion;
    }

    if ("GetInstanceId" in hostCall) {
      return instanceId;
    }

    if ("GetCurrentCulture" in hostCall) {
      return culture;
    }

    if ("GetCurrentUICulture" in hostCall) {
      return uiCulture;
    }

    // ===== Core output methods =====

    if ("Write1" in hostCall) {
      terminal.write(hostCall.Write1.params);
      return;
    }

    if ("Write2" in hostCall) {
      const [fg, bg, text] = hostCall.Write2.params;
      withColor(fg, bg, text);
      return;
    }

    if ("WriteLine1" in hostCall) {
      terminal.writeln("");
      return;
    }

    if ("WriteLine2" in hostCall) {
      terminal.writeln(hostCall.WriteLine2.params);
      return;
    }

    if ("WriteLine3" in hostCall) {
      const [fg, bg, text] = hostCall.WriteLine3.params;
      withColor(fg, bg, text + "\r\n");
      return;
    }

    if ("WriteErrorLine" in hostCall) {
      terminal.writeln(`Error: ${hostCall.WriteErrorLine.params}`);
      return;
    }

    if ("WriteDebugLine" in hostCall) {
      terminal.writeln(`Debug: ${hostCall.WriteDebugLine.params}`);
      return;
    }

    if ("WriteVerboseLine" in hostCall) {
      terminal.writeln(`Verbose: ${hostCall.WriteVerboseLine.params}`);
      return;
    }

    if ("WriteWarningLine" in hostCall) {
      terminal.writeln(`Warning: ${hostCall.WriteWarningLine.params}`);
      return;
    }

    if ("WriteProgress" in hostCall) {
      // Best-effort: show brief progress updates without trying to render a full UI.
      const { record } = hostCall.WriteProgress.params;
      if (record.activity || record.statusDescription) {
        terminal.writeln(
          `[progress] ${record.activity}: ${record.statusDescription}`,
        );
      }
      return;
    }

    // ===== Session / prompt control =====

    if ("SetShouldExit" in hostCall) {
      // Browser host has no process exit; higher-level code may disconnect.
      return;
    }

    if ("EnterNestedPrompt" in hostCall) {
      return;
    }

    if ("ExitNestedPrompt" in hostCall) {
      return;
    }

    if ("NotifyBeginApplication" in hostCall) {
      return;
    }

    if ("NotifyEndApplication" in hostCall) {
      return;
    }

    // ===== Input =====

    if ("ReadLine" in hostCall) {
      return await withHostCallInput(async () => {
        return await readLineFromTerminal({ echo: true });
      });
    }

    if ("ReadLineAsSecureString" in hostCall) {
      // Rust side accepts string and will UTF-16LE encode it as SecureString bytes.
      return await withHostCallInput(async () => {
        return await readLineFromTerminal({ echo: false });
      });
    }

    // ===== Credential prompts =====

    if ("PromptForCredential1" in hostCall) {
      const [caption, message, userName, _targetName] =
        hostCall.PromptForCredential1.params;
      return await withHostCallInput(async () => {
        return await promptCredentialFromTerminal({
          caption,
          message,
          defaultUserName: userName,
        });
      });
    }

    if ("PromptForCredential2" in hostCall) {
      const [caption, message, userName, _targetName] =
        hostCall.PromptForCredential2.params;
      return await withHostCallInput(async () => {
        return await promptCredentialFromTerminal({
          caption,
          message,
          defaultUserName: userName,
        });
      });
    }

    // ===== Choices =====

    if ("PromptForChoice" in hostCall) {
      const { caption, message, choices, defaultChoice } =
        hostCall.PromptForChoice.params;
      return await withHostCallInput(async () => {
        terminal.writeln(caption);
        if (message) terminal.writeln(message);
        for (let i = 0; i < choices.length; i++) {
          terminal.writeln(`${i}: ${choices[i]?.label ?? ""}`);
        }
        terminal.write(`Choice [default ${defaultChoice}]: `);
        const raw = (await readLineFromTerminal({ echo: true })).trim();
        if (raw.length === 0) return defaultChoice;
        const n = Number.parseInt(raw, 10);
        return Number.isFinite(n) ? n : defaultChoice;
      });
    }

    if ("PromptForChoiceMultipleSelection" in hostCall) {
      const { caption, message, choices, defaultChoices } =
        hostCall.PromptForChoiceMultipleSelection.params;
      return await withHostCallInput(async () => {
        terminal.writeln(caption);
        if (message) terminal.writeln(message);
        for (let i = 0; i < choices.length; i++) {
          terminal.writeln(`${i}: ${choices[i]?.label ?? ""}`);
        }
        terminal.write(
          `Choices (comma-separated) [default ${defaultChoices.join(",")}]: `,
        );
        const raw = (await readLineFromTerminal({ echo: true })).trim();
        if (raw.length === 0) return defaultChoices;
        const parsed = raw
          .split(",")
          .map((s: string) => Number.parseInt(s.trim(), 10))
          .filter((n: number) => Number.isFinite(n));
        return parsed.length > 0 ? parsed : defaultChoices;
      });
    }

    if ("Prompt" in hostCall) {
      const { caption, message, fields } = hostCall.Prompt.params;
      const out: Record<string, JsPsValue> = {};

      return await withHostCallInput(async () => {
        terminal.writeln(caption);
        if (message) terminal.writeln(message);

        for (const field of fields) {
          const label = field.label || field.name;
          terminal.writeln("");
          terminal.writeln(
            `${label}${field.isMandatory ? " (required)" : ""}`,
          );
          if (field.helpMessage) terminal.writeln(field.helpMessage);

          terminal.write("> ");
          const raw = isSecureStringType(field.parameterType)
            ? await readLineFromTerminal({ echo: false })
            : await readLineFromTerminal({ echo: true });
          if (isSecureStringType(field.parameterType)) terminal.writeln("");

          if (field.isMandatory && raw.trim().length === 0) {
            throw new Error(`Prompt field '${field.name}' is mandatory`);
          }
          out[field.name] = promptToJsPsValue(field, raw);
        }

        return out;
      });
    }

    // ===== Raw key input =====

    if ("GetKeyAvailable" in hostCall) {
      return keyQueue.length > 0;
    }

    if ("ReadKey" in hostCall) {
      const options = readKeyOptions(hostCall.ReadKey.params);
      return await withHostCallInput(async () => {
        const k = await readKeyAsync();

        if (!options.allowCtrlC && k.character === "\u0003") {
          throw new Error("ReadKey cancelled (Ctrl+C)");
        }

        if (!options.noEcho) {
          const ch = k.character;
          if (ch === "\r" || ch === "\n") {
            terminal.writeln("");
          } else if (ch !== "\u0000") {
            if (ch === "\b" || ch === "\u007f") {
              terminal.write("\b \b");
            } else if (ch.charCodeAt(0) >= 0x20) {
              terminal.write(ch);
            }
          }
        }

        // We always provide keyDown=true; IncludeKeyDown is effectively satisfied.
        // If a future version needs keyUp events, implement it here.
        if (!options.includeKeyDown) return k;
        return k;
      });
    }

    // ===== RawUI colors =====

    if ("GetForegroundColor" in hostCall) {
      return state.foregroundColor;
    }

    if ("SetForegroundColor" in hostCall) {
      state.foregroundColor = hostCall.SetForegroundColor.params;
      applyCurrentColors();
      return;
    }

    if ("GetBackgroundColor" in hostCall) {
      return state.backgroundColor;
    }

    if ("SetBackgroundColor" in hostCall) {
      state.backgroundColor = hostCall.SetBackgroundColor.params;
      applyCurrentColors();
      return;
    }

    // ===== Terminal / RawUI operations =====

    if ("SetCursorPosition" in hostCall) {
      const [x, y] = hostCall.SetCursorPosition.params;
      cursorTo(x, y);
      return;
    }

    if ("SetBufferContents1" in hostCall) {
      const { rect, cell } = hostCall.SetBufferContents1.params;
      if (rectCoversViewport(rect) && (cell.character === " " || cell.character === "\u0000")) {
        terminal.clear();
        return;
      }

      const ch = normalizeChar(cell.character) || " ";
      const width = Math.max(0, rect.right - rect.left + 1);
      const height = Math.max(0, rect.bottom - rect.top + 1);
      const line = ch.repeat(width);
      for (let yy = 0; yy < height; yy++) {
        cursorTo(rect.left, rect.top + yy);
        withColor(cell.foreground, cell.background, line);
      }
      return;
    }

    if ("SetBufferContents2" in hostCall) {
      const { rect, cell } = hostCall.SetBufferContents2.params;
      if (rectCoversViewport(rect) && (cell.character === " " || cell.character === "\u0000")) {
        terminal.clear();
        return;
      }

      const ch = normalizeChar(cell.character) || " ";
      const width = Math.max(0, rect.right - rect.left + 1);
      const height = Math.max(0, rect.bottom - rect.top + 1);
      const line = ch.repeat(width);
      for (let yy = 0; yy < height; yy++) {
        cursorTo(rect.left, rect.top + yy);
        withColor(cell.foreground, cell.background, line);
      }
      return;
    }

    if ("SetWindowTitle" in hostCall) {
      state.windowTitle = hostCall.SetWindowTitle.params;
      document.title = state.windowTitle;
      return;
    }

    if ("SetWindowPosition" in hostCall) {
      state.windowPosition = {
        x: hostCall.SetWindowPosition.params[0],
        y: hostCall.SetWindowPosition.params[1],
      };
      return;
    }

    if ("SetCursorSize" in hostCall) {
      state.cursorSize = hostCall.SetCursorSize.params;
      // Best-effort mapping: Windows cursor size is a percentage of cell height.
      // xterm.js supports styles, not an exact percentage.
      if (state.cursorSize >= 50) {
        terminal.options = { ...terminal.options, cursorStyle: "block" };
      } else {
        terminal.options = { ...terminal.options, cursorStyle: "underline" };
      }
      return;
    }

    if ("SetBufferSize" in hostCall) {
      const [w, h] = hostCall.SetBufferSize.params;
      if (w > 0 && h > 0) terminal.resize(w, h);
      return;
    }

    if ("SetWindowSize" in hostCall) {
      const [w, h] = hostCall.SetWindowSize.params;
      if (w > 0 && h > 0) terminal.resize(w, h);
      return;
    }

    if ("ScrollBufferContents" in hostCall) {
      const { source, destination, clip, fill } =
        hostCall.ScrollBufferContents.params;
      const blankFill = fill.character === " " || fill.character === "\u0000";

      // Best-effort: implement the common "scroll up/down within viewport" case
      // using ANSI scroll sequences + scroll region.
      //
      // PowerShell commonly uses ScrollBufferContents to shift the viewport by N
      // lines and fill the freed rows with blanks.
      const clipWidth = Math.max(0, clip.right - clip.left + 1);
      const clipHeight = Math.max(0, clip.bottom - clip.top + 1);
      const sourceHeight = Math.max(0, source.bottom - source.top + 1);
      const sourceWidth = Math.max(0, source.right - source.left + 1);

      const fullWidth =
        clipWidth >= terminal.cols &&
        sourceWidth >= terminal.cols &&
        clip.left <= 0 &&
        source.left <= 0;

      const sameClipSourceWidth =
        source.left === clip.left && source.right === clip.right;

      const dy = destination.y - source.top;

      const canScrollVertically =
        blankFill &&
        fullWidth &&
        sameClipSourceWidth &&
        destination.x === clip.left &&
        source.left === clip.left &&
        clip.top >= 0 &&
        clip.bottom >= clip.top;

      if (canScrollVertically) {
        // Scroll up by N: move lines [clip.top+N .. clip.bottom] -> [clip.top .. clip.bottom-N]
        if (
          dy < 0 &&
          destination.y === clip.top &&
          source.top === clip.top - dy &&
          sourceHeight === clipHeight + dy
        ) {
          const n = -dy;
          terminal.write(
            `\x1b[${sgrColor(fill.foreground, false)};${sgrColor(fill.background, true)}m`,
          );
          terminal.write(`\x1b[${clip.top + 1};${clip.bottom + 1}r`);
          terminal.write(`\x1b[${clip.bottom + 1};1H`);
          terminal.write(`\x1b[${n}S`);
          terminal.write(`\x1b[r`);
          terminal.write(
            `\x1b[${sgrColor(state.foregroundColor, false)};${sgrColor(state.backgroundColor, true)}m`,
          );
          return;
        }

        // Scroll down by N: move lines [clip.top .. clip.bottom-N] -> [clip.top+N .. clip.bottom]
        if (
          dy > 0 &&
          source.top === clip.top &&
          destination.y === clip.top + dy &&
          sourceHeight === clipHeight - dy
        ) {
          const n = dy;
          terminal.write(
            `\x1b[${sgrColor(fill.foreground, false)};${sgrColor(fill.background, true)}m`,
          );
          terminal.write(`\x1b[${clip.top + 1};${clip.bottom + 1}r`);
          terminal.write(`\x1b[${clip.top + 1};1H`);
          terminal.write(`\x1b[${n}T`);
          terminal.write(`\x1b[r`);
          terminal.write(
            `\x1b[${sgrColor(state.foregroundColor, false)};${sgrColor(state.backgroundColor, true)}m`,
          );
          return;
        }
      }

      // Fallback: if it's effectively a full clear, do it (common with Clear-Host).
      if (
        rectCoversViewport(source) &&
        rectCoversViewport(clip) &&
        destination.x === 0 &&
        destination.y === 0 &&
        blankFill
      ) {
        terminal.clear();
        return;
      }

      throw new Error(
        "ScrollBufferContents is only partially supported in the web terminal. Supported: vertical scrolling within full-width clip regions with blank fill.",
      );
    }

    if ("FlushInputBuffer" in hostCall) {
      keyQueue.length = 0;
      return;
    }

    // ===== Size / position queries =====

    if ("GetCursorSize" in hostCall) {
      return state.cursorSize;
    }

    if ("GetCursorPosition" in hostCall) {
      // Prefer xterm's live cursor when available, since ANSI output can move
      // the cursor without going through SetCursorPosition.
      const buf = terminal.buffer.active;
      return { x: buf.cursorX, y: buf.cursorY };
    }

    if ("GetWindowPosition" in hostCall) {
      return state.windowPosition;
    }

    if ("GetBufferSize" in hostCall) {
      return { width: terminal.cols, height: terminal.rows };
    }

    if ("GetWindowSize" in hostCall) {
      return { width: terminal.cols, height: terminal.rows };
    }

    if ("GetWindowTitle" in hostCall) {
      return state.windowTitle;
    }

    if ("GetMaxWindowSize" in hostCall) {
      return { width: terminal.cols, height: terminal.rows };
    }

    if ("GetMaxPhysicalWindowSize" in hostCall) {
      return { width: terminal.cols, height: terminal.rows };
    }

    if ("GetBufferContents" in hostCall) {
      const rect = hostCall.GetBufferContents.params.rect;

      const buf = terminal.buffer.active;
      const nullCell = buf.getNullCell();
      const viewportY = buf.viewportY;

      const out: Array<Array<{ character: string; foreground: number; background: number; flags: number }>> =
        [];

      const top = Math.max(0, rect.top);
      const left = Math.max(0, rect.left);
      const bottom = Math.max(top, rect.bottom);
      const right = Math.max(left, rect.right);

      for (let yy = top; yy <= bottom; yy++) {
        const line = buf.getLine(viewportY + yy);
        const row: Array<{ character: string; foreground: number; background: number; flags: number }> =
          [];

        for (let xx = left; xx <= right; xx++) {
          const cell = line?.getCell(xx, nullCell);
          if (!cell) {
            row.push({
              character: " ",
              foreground: state.foregroundColor,
              background: state.backgroundColor,
              flags: 0,
            });
            continue;
          }

          const width = cell.getWidth();
          const chars = width === 0 ? "\u0000" : cell.getChars();
          const ch = normalizeChar(Array.from(chars)[0] ?? " ");

          row.push({
            character: ch,
            foreground: consoleColorFromXtermCellFg(cell),
            background: consoleColorFromXtermCellBg(cell),
            flags: 0,
          });
        }

        out.push(row);
      }

      return out;
    }

    // ===== Interactive session methods (runspace stack) =====

    if ("PushRunspace" in hostCall) {
      state.runspaceStack.push(hostCall.PushRunspace.params.runspace);
      return;
    }

    if ("PopRunspace" in hostCall) {
      state.runspaceStack.pop();
      return;
    }

    if ("GetIsRunspacePushed" in hostCall) {
      return state.runspaceStack.length > 0;
    }

    if ("GetRunspace" in hostCall) {
      const top = state.runspaceStack[state.runspaceStack.length - 1];
      if (!top) return jsPsValueStr("");
      return top;
    }

    // ===== UNIMPLEMENTED - THROW ERRORS =====

    const methodName = Object.keys(hostCall)[0];
    throw new Error(
      `Unimplemented host call: ${methodName}. If you need this functionality, implement it in hostcall-handler.ts`
    );
  };

  return handlerImpl as unknown as TypedHostCallHandler;
}
