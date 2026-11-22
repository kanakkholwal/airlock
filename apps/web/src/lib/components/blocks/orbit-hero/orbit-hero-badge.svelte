<script lang="ts">
import type { ComponentProps } from "svelte";
import Badge from "#lib/components/ui/badge/badge.svelte";
import { cn } from "#lib/cn.js";
import { orbitHeroState } from "./context";

let { class: classProp, ...rest }: ComponentProps<typeof Badge> = $props();
const hero = orbitHeroState();
</script>

<!-- The pill above the headline. -->
<Badge variant="outline" {...rest} class={cn(hero.styles.badge(), classProp)} />
