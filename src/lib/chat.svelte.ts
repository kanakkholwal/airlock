// Mirrors the Rust `Part` enum (src-tauri/src/chat/store.rs) until bindings.ts exports it.
export type Part =
  | { type: "text"; text: string }
  | { type: "reasoning"; text: string }
  | { type: "tool_call"; call_id: string; name: string; arguments: string }
  | { type: "tool_result"; call_id: string; output: string; is_error: boolean }
  | { type: "image"; path: string }
  | { type: "audio"; path: string };

export type ChatMessage = {
  id: number;
  role: "user" | "assistant";
  parts: Part[];
  status: "queued" | "streaming" | "done" | "error";
};

export type EngineState = "unavailable" | "loading" | "ready" | "error";

/** Streams a reply; resolves when finished. Throws AbortError when cancelled. */
export type ChatTransport = (
  history: ChatMessage[],
  onText: (delta: string) => void,
  signal: AbortSignal,
) => Promise<void>;

class ChatSession {
  messages = $state<ChatMessage[]>([]);
  engine = $state<EngineState>("unavailable");
  private nextId = 1;
  private transport: ChatTransport | null = null;
  private abort: AbortController | null = null;

  get busy() {
    return this.abort !== null;
  }

  get queuedCount() {
    return this.messages.filter((m) => m.status === "queued").length;
  }

  attach(transport: ChatTransport | null, state: EngineState) {
    this.transport = transport;
    this.engine = state;
    if (state === "ready") void this.drain();
  }

  send(text: string) {
    this.messages.push({
      id: this.nextId++,
      role: "user",
      parts: [{ type: "text", text }],
      status: this.engine === "ready" ? "done" : "queued",
    });
    if (this.engine === "ready") void this.drain();
  }

  stop() {
    this.abort?.abort();
  }

  // Sends queued user messages once the engine is ready (plan section 14).
  private async drain() {
    if (this.busy || !this.transport) return;
    const pending = this.messages.some((m) => m.role === "user" && m.status === "queued");
    const last = this.messages.at(-1);
    if (!pending && last?.role !== "user") return;
    for (const m of this.messages) if (m.status === "queued") m.status = "done";

    const reply: ChatMessage = {
      id: this.nextId++,
      role: "assistant",
      parts: [{ type: "text", text: "" }],
      status: "streaming",
    };
    this.messages.push(reply);
    const live = this.messages[this.messages.length - 1];
    this.abort = new AbortController();
    try {
      await this.transport(
        this.messages.slice(0, -1),
        (delta) => {
          const part = live.parts[0];
          if (part.type === "text") part.text += delta;
        },
        this.abort.signal,
      );
      live.status = "done";
    } catch (e) {
      live.status = (e as Error).name === "AbortError" ? "done" : "error";
    } finally {
      this.abort = null;
    }
  }
}

export const chat = new ChatSession();
