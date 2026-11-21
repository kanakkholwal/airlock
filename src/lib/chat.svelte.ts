import { commands, type Answer, type Conversation, type Message as DbMessage, type Part, type Risk } from "../bindings";

export type { Part };

export type ChatMessage = {
  id: number;
  role: "user" | "assistant";
  parts: Part[];
  status: "queued" | "streaming" | "done" | "error";
  error?: string;
  /** Seconds spent reasoning, shown on the collapsed reasoning panel. */
  thinkingSeconds?: number;
  tokensPerSecond?: number;
  /** Token counts the server reported for this reply. */
  promptTokens?: number;
  completionTokens?: number;
  firstTokenMs?: number;
  /** Made in this session by sending or regenerating; messages loaded from history are not. */
  fresh?: boolean;
};

export type EngineState = "unavailable" | "loading" | "ready" | "error";

export type Delta = "text" | "reasoning";

export type Usage = {
  tokensPerSecond: number | null;
  promptTokens: number | null;
  completionTokens: number | null;
};

/** A step of the assistant using a tool, in the order it happens. */
export type ToolEvent =
  | { type: "call"; callId: string; name: string; arguments: string; title: string }
  | { type: "approval"; callId: string; name: string; title: string; risk: Risk }
  | { type: "result"; callId: string; output: string; isError: boolean };

/** A tool call waiting for a yes or no. */
export type PendingApproval = { callId: string; name: string; title: string; risk: Risk };

/** Streams a reply; resolves when finished. Throws AbortError when cancelled. */
export type ChatTransport = (
  history: ChatMessage[],
  onDelta: (kind: Delta, text: string) => void,
  signal: AbortSignal,
  onTool: (event: ToolEvent) => void,
) => Promise<Usage>;

export type ChatStats = {
  replies: number;
  /** Tokens in the context after the latest reply, or null before any reply this session. */
  contextTokens: number | null;
  lastPromptTokens: number | null;
  lastCompletionTokens: number | null;
  lastTokensPerSecond: number | null;
  averageTokensPerSecond: number | null;
  lastFirstTokenMs: number | null;
  totalCompletionTokens: number;
};

const inTauri = () => typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

const titleFrom = (text: string) => {
  const line = text.trim().split("\n")[0];
  return line.length > 48 ? `${line.slice(0, 47)}…` : line || "New chat";
};

const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));

export const textOf = (m: ChatMessage) =>
  m.parts.flatMap((p) => (p.type === "text" ? [p.text] : [])).join("");

export const reasoningOf = (m: ChatMessage) =>
  m.parts.flatMap((p) => (p.type === "reasoning" ? [p.text] : [])).join("");

/** The result of one tool call in a message, once it has come back. */
export const resultOf = (m: ChatMessage, callId: string) =>
  m.parts.find((p): p is Extract<Part, { type: "tool_result" }> => p.type === "tool_result" && p.call_id === callId);

export const hasTools = (m: ChatMessage) => m.parts.some((p) => p.type === "tool_call");

export const imagesOf = (m: ChatMessage) =>
  m.parts.flatMap((p) => (p.type === "image" ? [p.path] : []));

export type ImageRef = { path: string; name: string };

/** A library item a message points at: its id, title and a short preview, never the whole text. */
export type Reference = { id: string; title: string; kind: string; preview: string };

export const referencesOf = (m: ChatMessage): Reference[] =>
  m.parts.flatMap((p) => (p.type === "reference" ? [{ id: p.id, title: p.title, kind: p.kind, preview: p.preview }] : []));

class ChatSession {
  messages = $state<ChatMessage[]>([]);
  conversations = $state<Conversation[]>([]);
  conversationId = $state<number | null>(null);
  engine = $state<EngineState>("unavailable");
  /** Set when a save fails, so nothing is lost silently. */
  saveError = $state<string | null>(null);
  /** True once the saved chat list has been read (or there is no backend to read from). */
  listLoaded = $state(false);
  /** True while a saved conversation is being read from disk. */
  opening = $state(false);
  /** What each tool call is doing in words; lives only for this session. */
  toolTitles = $state<Record<string, string>>({});
  /** Tool calls waiting for the person, by call id. */
  approvals = $state<Record<string, PendingApproval>>({});
  /** Set by the engine store: asks it to load a model when a message needs one. */
  needModel: (() => void) | null = null;
  private nextId = 1;
  private transport: ChatTransport | null = null;
  private abort: AbortController | null = null;
  private modelId: string | null = null;
  // Serialises writes so messages land in order.
  private writes: Promise<unknown> = Promise.resolve();
  // Conversations created this session whose title is still the first-line fallback.
  private needsTitle = new Set<number>();
  private titleJobs: { id: number; user: string; reply: string }[] = [];
  private titling = false;

  get busy() {
    return this.abort !== null;
  }

  get queuedCount() {
    return this.messages.filter((m) => m.status === "queued").length;
  }

  /** Derived from what the server reported; unknown values stay null, never guessed. */
  get stats(): ChatStats {
    const done = this.messages.filter((m) => m.role === "assistant" && m.completionTokens !== undefined);
    const last = done.at(-1);
    const speeds = done.flatMap((m) => (m.tokensPerSecond ? [m.tokensPerSecond] : []));
    return {
      replies: done.length,
      contextTokens: last ? (last.promptTokens ?? 0) + (last.completionTokens ?? 0) : null,
      lastPromptTokens: last ? (last.promptTokens ?? 0) : null,
      lastCompletionTokens: last ? (last.completionTokens ?? 0) : null,
      lastTokensPerSecond: last?.tokensPerSecond ?? null,
      averageTokensPerSecond: speeds.length ? speeds.reduce((a, b) => a + b, 0) / speeds.length : null,
      lastFirstTokenMs: last?.firstTokenMs ?? null,
      totalCompletionTokens: done.reduce((n, m) => n + (m.completionTokens ?? 0), 0),
    };
  }

  attach(transport: ChatTransport | null, state: EngineState, modelId: string | null = null) {
    this.transport = transport;
    this.engine = state;
    this.modelId = modelId;
    if (state === "ready") void this.drain();
  }

  async init() {
    await this.refreshList();
  }

  async refreshList() {
    if (!inTauri()) {
      this.listLoaded = true;
      return;
    }
    const res = await commands.chatListConversations();
    if (res.status === "ok") this.conversations = res.data;
    this.listLoaded = true;
  }

  newChat() {
    this.stop();
    this.messages = [];
    this.conversationId = null;
  }

  async open(id: number) {
    if (!inTauri() || id === this.conversationId) return;
    this.stop();
    this.opening = true;
    const res = await commands.chatLoadMessages(id);
    this.opening = false;
    if (res.status === "error") {
      this.saveError = res.error;
      return;
    }
    this.conversationId = id;
    this.messages = res.data.flatMap((m: DbMessage): ChatMessage[] =>
      m.role === "user" || m.role === "assistant"
        ? [{ id: this.nextId++, role: m.role, parts: m.parts, status: "done" }]
        : [],
    );
  }

  async rename(id: number, title: string) {
    const clean = title.trim();
    if (!inTauri() || !clean) return;
    const res = await commands.chatRenameConversation(id, clean);
    if (res.status === "error") this.saveError = res.error;
    await this.refreshList();
  }

  get currentTitle() {
    return this.conversations.find((c) => c.id === this.conversationId)?.title ?? null;
  }

  async remove(id: number) {
    if (!inTauri()) return;
    const res = await commands.chatDeleteConversation(id);
    if (res.status === "error") {
      this.saveError = res.error;
      return;
    }
    if (this.conversationId === id) this.newChat();
    await this.refreshList();
  }

  send(text: string, images: ImageRef[] = [], references: Reference[] = []) {
    const parts: Part[] = [
      ...images.map((i): Part => ({ type: "image", path: i.path })),
      ...references.map((r): Part => ({ type: "reference", id: r.id, title: r.title, kind: r.kind, preview: r.preview })),
      { type: "text", text },
    ];
    const msg: ChatMessage = {
      id: this.nextId++,
      role: "user",
      parts,
      status: this.engine === "ready" ? "done" : "queued",
      fresh: true,
    };
    this.messages.push(msg);
    this.save("user", $state.snapshot(parts), text);
    if (this.engine === "ready") void this.drain();
    else if (this.engine !== "loading") this.needModel?.();
  }

  /** Replaces the last assistant reply with a new one. */
  regenerate() {
    if (this.busy) return;
    const last = this.messages.at(-1);
    if (last?.role !== "assistant") return;
    this.messages.pop();
    this.unsaveLast();
    if (this.engine === "ready") void this.drain();
  }

  stop() {
    this.abort?.abort();
  }

  private unsaveLast() {
    if (!inTauri() || this.conversationId === null) return;
    const id = this.conversationId;
    this.writes = this.writes.then(async () => {
      try {
        const res = await commands.chatDeleteLastMessage(id);
        if (res.status === "error") this.saveError = res.error;
      } catch (e) {
        this.saveError = e instanceof Error ? e.message : String(e);
      }
    });
  }

  /** Appends to the open conversation, creating it on the first message. */
  private save(role: "user" | "assistant", parts: Part[], titleSource?: string) {
    if (!inTauri()) return Promise.resolve();
    const run = this.writes.then(async () => {
      try {
        if (this.conversationId === null) {
          const created = await commands.chatCreateConversation(
            titleFrom(titleSource ?? ""),
            this.modelId,
          );
          if (created.status === "error") throw new Error(created.error);
          this.conversationId = created.data;
          this.needsTitle.add(created.data);
        }
        const res = await commands.chatAppendMessage(this.conversationId, role, parts);
        if (res.status === "error") throw new Error(res.error);
        this.saveError = null;
        await this.refreshList();
      } catch (e) {
        this.saveError = e instanceof Error ? e.message : String(e);
      }
    });
    this.writes = run;
    return run;
  }

  // Decision (product): the title is a second, queued model call after the first reply, so
  // it never competes with the reply being written. If it fails the first-line title stays.
  private queueTitle(id: number) {
    const first = this.messages.find((m) => m.role === "user");
    const reply = this.messages.find((m) => m.role === "assistant");
    if (!first || !reply) return;
    this.titleJobs.push({ id, user: textOf(first), reply: textOf(reply) });
    void this.runTitles();
  }

  private async runTitles() {
    if (this.titling) return;
    this.titling = true;
    try {
      for (let job = this.titleJobs.shift(); job; job = this.titleJobs.shift()) {
        while (this.busy) await sleep(300);
        if (this.engine !== "ready") continue;
        const res = await commands.chatTitle(job.user, job.reply);
        if (res.status === "ok" && res.data) {
          await commands.chatRenameConversation(job.id, res.data);
          await this.refreshList();
        }
      }
    } catch {
      // Keep the fallback title; a failed title is not worth interrupting anyone.
    } finally {
      this.titling = false;
    }
  }

  // Sends queued user messages once the engine is ready (plan section 14).
  /** Sends the person's answer on an approval card; the assistant carries on from there. */
  async answerTool(callId: string, answer: Answer) {
    delete this.approvals[callId];
    await commands.agentDecide(callId, answer);
  }

  private async drain() {
    if (this.busy || !this.transport) return;
    const hasQueued = this.messages.some((m) => m.role === "user" && m.status === "queued");
    const last = this.messages.at(-1);
    if (!hasQueued && last?.role !== "user") return;
    for (const m of this.messages) if (m.status === "queued") m.status = "done";

    this.messages.push({ id: this.nextId++, role: "assistant", parts: [], status: "streaming", fresh: true });
    const live = this.messages[this.messages.length - 1];
    this.abort = new AbortController();

    const started = performance.now();
    let reasoningEnded = 0;
    let firstToken = 0;
    // Decision (product): tokens are applied once per frame, not once per token, so fast
    // models do not re-render the message hundreds of times a second.
    let buffered: { kind: Delta; text: string }[] = [];
    let frame = 0;
    const append = (kind: Delta, text: string) => {
      const tail = live.parts.at(-1);
      if (tail && tail.type === kind) tail.text += text;
      else live.parts.push({ type: kind, text });
    };
    const flush = () => {
      frame = 0;
      for (const d of buffered) append(d.kind, d.text);
      buffered = [];
    };

    const onTool = (e: ToolEvent) => {
      // Text that came before the tool must land first, so the order on screen is the real one.
      flush();
      if (e.type === "call") {
        this.toolTitles[e.callId] = e.title;
        live.parts.push({ type: "tool_call", call_id: e.callId, name: e.name, arguments: e.arguments });
      } else if (e.type === "approval") {
        this.approvals[e.callId] = { callId: e.callId, name: e.name, title: e.title, risk: e.risk };
      } else {
        delete this.approvals[e.callId];
        live.parts.push({ type: "tool_result", call_id: e.callId, output: e.output, is_error: e.isError });
      }
    };

    try {
      const result = await this.transport(
        this.messages.slice(0, -1),
        (kind, text) => {
          const now = performance.now();
          if (!firstToken) firstToken = now;
          if (kind === "text" && !reasoningEnded) reasoningEnded = now;
          buffered.push({ kind, text });
          if (!frame) frame = requestAnimationFrame(flush);
        },
        this.abort.signal,
        onTool,
      );
      live.tokensPerSecond = result.tokensPerSecond ?? undefined;
      live.promptTokens = result.promptTokens ?? undefined;
      live.completionTokens = result.completionTokens ?? undefined;
      live.status = "done";
    } catch (e) {
      if ((e as Error).name === "AbortError") {
        live.status = "done";
      } else {
        live.status = "error";
        live.error = e instanceof Error ? e.message : String(e);
      }
    } finally {
      if (frame) cancelAnimationFrame(frame);
      flush();
      this.abort = null;
    }
    if (firstToken) live.firstTokenMs = Math.round(firstToken - started);
    if (live.parts.some((p) => p.type === "reasoning")) {
      const end = reasoningEnded || performance.now();
      live.thinkingSeconds = Math.max(1, Math.round((end - started) / 1000));
    }
    // A partial reply from a stopped stream is kept; an empty one is not worth saving.
    if (live.status === "done" && live.parts.length > 0) {
      void this.save("assistant", $state.snapshot(live.parts)).then(() => {
        const id = this.conversationId;
        if (id !== null && this.needsTitle.delete(id)) this.queueTitle(id);
      });
    }
  }
}

export const chat = new ChatSession();
