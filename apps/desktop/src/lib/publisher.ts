/** `Qwen/Qwen3-4B-GGUF` splits into the publisher and the model name. */
export const publisherOf = (id: string) => (id.includes("/") ? id.split("/")[0] : "");
export const modelNameOf = (id: string) => id.split("/").pop() ?? id;
/** A publisher is an organization or a person; Hugging Face serves both at the same path. */
export const publisherPageUrl = (name: string) => `https://huggingface.co/${name}`;
export const hfUrl = (id: string) => `https://huggingface.co/${id}`;

/** A steady hue from a name, so one publisher always gets one colour without fetching its logo. */
export function hueOf(name: string): number {
  let h = 0;
  for (const c of name.toLowerCase()) h = (h * 31 + c.charCodeAt(0)) % 360;
  return h;
}
