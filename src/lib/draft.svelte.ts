import { convertFileSrc } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { commands } from "../bindings";

export type DraftImage = { path: string; name: string; sizeBytes: number; src: string };

const inTauri = () => typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

/** The message being written: text plus images waiting to be sent. Cleared on send. */
class Draft {
  text = $state("");
  images = $state<DraftImage[]>([]);
  /** Files being validated and copied, shown as uploading chips. */
  pending = $state<string[]>([]);
  error = $state<string | null>(null);

  get canSend() {
    return this.text.trim().length > 0 || this.images.length > 0;
  }

  /** Opens the file picker; each chosen image is validated and copied by the backend. */
  async pickImages() {
    if (!inTauri()) return;
    const picked = await open({
      multiple: true,
      filters: [{ name: "Images", extensions: ["png", "jpg", "jpeg", "gif", "webp"] }],
    });
    const paths = Array.isArray(picked) ? picked : picked ? [picked] : [];
    this.error = null;
    this.pending = paths.map((p) => p.split(/[\\/]/).pop() ?? "image");
    for (const path of paths) {
      const res = await commands.chatAttachImage(path);
      if (res.status === "error") {
        this.error = res.error;
        continue;
      }
      const { path: stored, name, size_bytes } = res.data;
      if (!this.images.some((i) => i.path === stored)) {
        this.images.push({ path: stored, name, sizeBytes: size_bytes, src: convertFileSrc(stored) });
      }
    }
    this.pending = [];
  }

  remove(path: string) {
    this.images = this.images.filter((i) => i.path !== path);
  }

  clear() {
    this.text = "";
    this.images = [];
    this.error = null;
  }
}

export const draft = new Draft();
