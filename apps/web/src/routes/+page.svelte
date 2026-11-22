<script lang="ts">
  import { Footer } from "#lib/components/blocks/footer/index.js";
  import {
    FaqSection,
    FaqSectionHeader,
    FaqSectionList,
    FaqSectionTitle,
  } from "#lib/components/blocks/faq-section/index.js";
  import Download from "#lib/site/Download.svelte";
  import Hero from "#lib/site/Hero.svelte";
  import Logo from "#lib/site/Logo.svelte";
  import Models from "#lib/site/Models.svelte";
  import SiteHeader from "#lib/site/SiteHeader.svelte";
  import Privacy from "#lib/site/Privacy.svelte";
  import Reel from "#lib/site/Reel.svelte";
  import { faq, footerColumns } from "#lib/site/content.js";
</script>

<div class="relative z-10">
  <SiteHeader />

  <main>
    <Hero />
    <Reel />
    <Privacy />
    <Models />

    <section id="faq" class="mx-auto w-full max-w-4xl scroll-mt-24 px-6 py-24" aria-labelledby="faq-title">
      <FaqSection items={faq} variant="accordion">
        <FaqSectionHeader>
          <FaqSectionTitle id="faq-title">Questions</FaqSectionTitle>
        </FaqSectionHeader>
        <FaqSectionList />
      </FaqSection>
    </section>

    <Download />
  </main>

  {#snippet brand()}
    <span class="flex items-center gap-2 font-heading text-lg font-bold text-foreground"><Logo size={24} />Airlock Studio</span>
  {/snippet}
  {#snippet copyright()}
    Airlock Studio by Nexonauts. Free and open source.
  {/snippet}
  <Footer layout="notched" topHref="#top" topLabel="Back to top" description="Private AI on your own computer." columns={footerColumns} {brand} {copyright} />
</div>
