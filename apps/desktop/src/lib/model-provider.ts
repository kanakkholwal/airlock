// Decision (product): models are grouped by who made them, not by file. The catalog names the
// family in the id or name, so a small table is enough and an unknown family lands under Other.
const FAMILIES: { match: RegExp; provider: string }[] = [
  { match: /^(lfm|liquid)/i, provider: "Liquid AI" },
  { match: /^qwen/i, provider: "Qwen" },
  { match: /^(gemma|gemini)/i, provider: "Google" },
  { match: /^(llama|meta)/i, provider: "Meta" },
  { match: /^(gpt|openai)/i, provider: "OpenAI" },
  { match: /^(mistral|ministral|mixtral)/i, provider: "Mistral" },
  { match: /^phi/i, provider: "Microsoft" },
  { match: /^deepseek/i, provider: "DeepSeek" },
  { match: /^(smollm|smolvlm)/i, provider: "Hugging Face" },
];

export const OTHER_PROVIDER = "Other";

export function providerOf(model: { id: string; name: string }): string {
  const hit = FAMILIES.find((f) => f.match.test(model.id) || f.match.test(model.name));
  return hit?.provider ?? OTHER_PROVIDER;
}

/** Groups in the order providers first appear, with Other last, keeping each model's order. */
export function groupByProvider<T extends { provider?: string }>(items: readonly T[]): { provider: string; items: T[] }[] {
  const groups: { provider: string; items: T[] }[] = [];
  for (const item of items) {
    const provider = item.provider ?? OTHER_PROVIDER;
    const group = groups.find((g) => g.provider === provider);
    if (group) group.items.push(item);
    else groups.push({ provider, items: [item] });
  }
  return groups.sort((a, b) => Number(a.provider === OTHER_PROVIDER) - Number(b.provider === OTHER_PROVIDER));
}
