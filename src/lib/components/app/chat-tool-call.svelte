<script lang="ts">
  import type { Part } from "../../../bindings";
  import { chat, type PendingApproval } from "#lib/chat.svelte.js";
  import {
    ApprovalGate,
    ApprovalGateActions,
    ApprovalGateArgs,
    ApprovalGateBody,
    ApprovalGateHeader,
    ApprovalGateReceipt,
    ApprovalGateSummary,
    ApprovalGateTitle,
  } from "#lib/components/agents/approval-gate/index.js";
  import { Tool, ToolContent, ToolCopy, ToolActions, ToolInput, ToolOutput, ToolTrigger } from "#lib/components/agents/tool/index.js";

  // Decision (product): every step the assistant takes with a tool is visible, in order, in the
  // reply: what it did in words, what it sent and what came back. A step that needs a yes opens
  // its approval card right there, so the question and the action are never apart.
  let {
    call,
    result,
    approval,
  }: {
    call: Extract<Part, { type: "tool_call" }>;
    result?: Extract<Part, { type: "tool_result" }>;
    approval?: PendingApproval;
  } = $props();

  const title = $derived(chat.toolTitles[call.call_id] ?? call.name.replaceAll("_", " "));
  const status = $derived(approval ? "approval" : result ? (result.is_error ? "error" : "done") : "running");
  const kind = $derived(call.name.includes("search") ? "search" : "function");
  const args = $derived.by(() => {
    try {
      const v = JSON.parse(call.arguments);
      return v && typeof v === "object" && !Array.isArray(v) ? (v as Record<string, never>) : {};
    } catch {
      return {};
    }
  });
  const pretty = $derived(Object.keys(args).length ? JSON.stringify(args, null, 2) : "No arguments");
  const risk = $derived(approval?.risk === "write" ? "medium" : "low");
  const summary = $derived(
    approval?.risk === "ask"
      ? "This shares private text with the assistant."
      : approval?.risk === "write"
        ? "This saves something on your computer."
        : "",
  );
</script>

<Tool {status} variant="plain">
  <ToolTrigger name={call.name} {title} {kind} />
  <ToolContent>
    {#if approval}
      <ApprovalGate
        variant="plain"
        status="pending"
        now={Date.now()}
        request={{ tool: call.name, title: approval.title, summary, risk, args }}
      >
        <ApprovalGateReceipt />
        <ApprovalGateBody>
          <ApprovalGateHeader />
          <ApprovalGateTitle />
          <ApprovalGateSummary />
          <ApprovalGateArgs />
          <ApprovalGateActions
            onApprove={() => chat.answerTool(call.call_id, "once")}
            onAlwaysAllow={() => chat.answerTool(call.call_id, "always")}
            onDeny={() => chat.answerTool(call.call_id, "deny")}
          />
        </ApprovalGateBody>
      </ApprovalGate>
    {/if}
    <ToolInput>{pretty}</ToolInput>
    {#if result}
      <ToolOutput>{result.output}</ToolOutput>
      <ToolActions><ToolCopy text={result.output} /></ToolActions>
    {/if}
  </ToolContent>
</Tool>
