<script lang="ts">
import Progress from "#lib/components/ui/progress/progress.svelte";
import { getTranscriptRow } from "./context";

/** Shown while the job runs. */
let { value, label }: { value: number; label: string } = $props();

const row = getTranscriptRow();
</script>

{#if row.status === "running"}
	<Progress size="sm" {value} {label} helper={`${Math.round(value)}%`} />
{/if}
