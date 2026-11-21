<script lang="ts">
  import type { Component, Snippet } from "svelte";
  import {
    Empty,
    EmptyContent,
    EmptyDescription,
    EmptyHeader,
    EmptyMedia,
    EmptyTitle,
  } from "#lib/components/ui/empty/index.js";

  // Decision (product): empty, error and offline screens always say what happened, why it
  // matters, and what to do next. One component so they all look and behave the same.
  let {
    icon: Icon,
    tone = "neutral",
    title,
    description,
    actions,
    extra,
    variant = "default",
    size = "default",
  }: {
    icon: Component<{ size?: number | string }>;
    tone?: "neutral" | "primary" | "success" | "warning" | "destructive" | "info";
    title: string;
    description?: string;
    actions?: Snippet;
    /** Extra content under the actions, such as technical details. */
    extra?: Snippet;
    variant?: "default" | "card" | "outline";
    size?: "sm" | "default" | "lg";
  } = $props();
</script>

<Empty {variant} {size}>
  <EmptyHeader>
    <EmptyMedia variant="icon" {tone}><Icon size={20} /></EmptyMedia>
    <EmptyTitle>{title}</EmptyTitle>
    {#if description}<EmptyDescription>{description}</EmptyDescription>{/if}
  </EmptyHeader>
  {#if actions}<EmptyContent>{@render actions()}</EmptyContent>{/if}
  {#if extra}{@render extra()}{/if}
</Empty>
