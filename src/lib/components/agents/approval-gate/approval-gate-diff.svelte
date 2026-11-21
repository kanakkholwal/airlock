<script lang="ts">
import FileDiff from "#lib/components/ui/file-diff/file-diff.svelte";
import { approvalGateState } from "./context";

let { class: classProp }: { class?: string } = $props();

const ctx = approvalGateState("ApprovalGateDiff");
</script>

<!-- The proposed file change, when the request carries one. -->
{#if ctx.request.diff}
	<FileDiff filename={ctx.request.diff.filename} lines={ctx.request.diff.lines} class={classProp} />
{/if}
