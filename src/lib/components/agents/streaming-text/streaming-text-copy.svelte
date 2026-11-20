<script lang="ts">
import CopyButton from "#lib/components/ui/copy-button/copy-button.svelte";
import { cn } from "#lib/cn.js";
import { streamingTextState } from "./context";

let { label = "Copy", class: classProp }: { label?: string; class?: string } = $props();
const ctx = streamingTextState("StreamingTextCopy");
</script>

<!-- Copies the whole answer as plain text. -->
<CopyButton data-slot="streaming-text-copy" text={ctx.text} {label} iconOnly class={cn(ctx.styles.copy(), classProp)} />
