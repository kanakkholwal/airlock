<script lang="ts">
  import { Button } from "$lib/components/ui/button/index.js";
  import { Card, CardContent, CardHeader, CardTitle } from "$lib/components/ui/card/index.js";
  import { Input } from "$lib/components/ui/input/index.js";
</script>

<main class="mx-auto max-w-md p-6">
  <Card>
    <CardHeader><CardTitle>Airlock</CardTitle></CardHeader>
    <CardContent class="flex gap-2">
      <Input placeholder="Say something" />
      <Button>Send</Button>
    </CardContent>
  </Card>
</main>
