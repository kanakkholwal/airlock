<script lang="ts">
import type { ComponentProps } from "svelte";
import Empty from "../../ui/empty/empty.svelte";
import { cn } from "../../../cn.js";

let { class: classProp, ...rest }: ComponentProps<typeof Empty> = $props();
</script>

<!-- Placeholder before the first turn: an Empty, so compose it from EmptyHeader, EmptyTitle and friends. -->
<Empty data-slot="conversation-empty" size="sm" {...rest} class={cn("min-h-48", classProp)} />
