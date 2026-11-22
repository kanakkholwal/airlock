<script lang="ts">
  import { Button } from "#lib/components/ui/button/index.js";
  import { Footer } from "#lib/components/blocks/footer/index.js";
  import {
    FaqSection,
    FaqSectionHeader,
    FaqSectionList,
    FaqSectionTitle,
  } from "#lib/components/blocks/faq-section/index.js";
  import Download from "#lib/site/Download.svelte";
  import Hero from "#lib/site/Hero.svelte";
  import OpenSource from "#lib/site/OpenSource.svelte";
  import SeeItWork from "#lib/site/SeeItWork.svelte";
  import Logo from "#lib/site/Logo.svelte";
  import Mascot from "#lib/site/Mascot.svelte";
  import { inView } from "#lib/site/look.js";
  import Buddies from "#lib/site/Buddies.svelte";
  import { pick } from "#lib/site/buddies.js";
  import Models from "#lib/site/Models.svelte";
  import SiteHeader from "#lib/site/SiteHeader.svelte";
  import Privacy from "#lib/site/Privacy.svelte";
  import { faq, footerColumns } from "#lib/site/content.js";

  const foot = pick("footer", 2);
</script>

<div class="relative z-10">
  <SiteHeader />

  <main>
    <Hero />
    <SeeItWork />
    <Privacy />
    <Models />
    <OpenSource />

    <!-- Four characters keep the questions company. -->
    <div class="relative">
      <Buddies section="faq" spots={[
        "absolute top-20 left-[6%] hidden w-24 lg:block",
        "absolute top-10 right-[7%] hidden w-24 lg:block",
        "absolute bottom-40 left-[9%] hidden w-24 lg:block",
        "absolute right-[9%] bottom-28 hidden w-24 lg:block",
      ]} />
      <section id="faq" class="mx-auto w-full max-w-3xl scroll-mt-24 px-6 py-24" aria-labelledby="faq-title">
        <FaqSection items={faq} variant="accordion">
          <FaqSectionHeader>
            <FaqSectionTitle id="faq-title">Questions, answered</FaqSectionTitle>
          </FaqSectionHeader>
          <FaqSectionList />
        </FaqSection>
      </section>
    </div>

    <Download />
  </main>


  {#snippet brand()}
    <span class="flex items-center gap-2 font-heading text-lg font-bold text-foreground"><Logo size={24} />Airlock Studio</span>
  {/snippet}
  {#snippet copyright()}
    Airlock Studio by Nexonauts. Free and open source.
  {/snippet}
  {#snippet actions()}
    <Button variant="ghost" size="sm" href="#top" class="rounded-full! text-muted-foreground">Back to top ↑</Button>
  {/snippet}

  <!-- Framed like the navbar: a rim and a soft shadow around an inner sheet. The two characters sit
       behind the frame and rise over its top edge once, when the footer scrolls into view. -->
  <div class="peek relative mx-auto mt-24 w-full max-w-6xl px-3 pb-3 sm:px-5" use:inView>
    <Mascot kind={foot[0].kind} tone={foot[0].tone} class="peek-item absolute bottom-[calc(100%-1.75rem)] left-10 z-0 w-20 sm:left-20 sm:w-24" />
    <Mascot kind={foot[1].kind} tone={foot[1].tone} class="peek-item absolute right-10 bottom-[calc(100%-1.75rem)] z-0 w-20 [transition-delay:120ms] sm:right-20 sm:w-24" />
    <div class="relative z-10 rounded-[2rem] bg-card p-1 shadow-(--surface-shadow)">
      <div class="rounded-[1.75rem] bg-background">
        <Footer layout="split" class="rounded-[1.75rem] border-0" description="AI that stays on your laptop." columns={footerColumns} {brand} {copyright} {actions} />
      </div>
    </div>
  </div>
</div>
