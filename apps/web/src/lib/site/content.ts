import {
  ClipboardIcon,
  CpuIcon,
  DownloadMinimalisticIcon,
  EyeIcon,
  GlobalIcon,
  LockKeyholeIcon,
  MagicWandIcon,
  ShieldCheckIcon,
} from "@solar-icons/svelte/linear";
import type { Component } from "svelte";

// Decision (product): the page reads from this data. A new tool, question, picture or link is a
// line here, and the sections render whatever is listed.
export const RELEASES = "https://github.com/kanakkholwal/airlock/releases";
export const REPO = "https://github.com/kanakkholwal/airlock";

// What can leave the computer, taken from Settings, Privacy in the app.
export const ledger = [
  { what: "Chats, images and voice", where: "This laptop", when: "Never leaves" },
  { what: "Model search", where: "Hugging Face", when: "When you search" },
  { what: "Model downloads", where: "Hugging Face", when: "When you press Download" },
  { what: "Engine downloads", where: "GitHub", when: "When you press Install" },
  { what: "Update checks", where: "GitHub", when: "Version number only" },
];

export const faq = [
  { question: "Is it really free?", answer: "Yes, and the code is open. Each model has its own license, shown before you download it." },
  { question: "Will it run on my laptop?", answer: "Small models run on a 4 GB graphics card. Without one everything still works, only slower." },
  { question: "Does it work offline?", answer: "Yes, once a model is downloaded. It goes online only to search or download models, or to check for updates." },
  { question: "Why does Windows warn me?", answer: "This early build is not code-signed yet, so Windows asks you to confirm before it runs." },
  { question: "Where does my data live?", answer: "In a folder on your laptop: chats, recordings, pictures and settings. Nothing is uploaded." },
  { question: "Can I use other models?", answer: "Yes. Any GGUF model from Hugging Face." },
];

export type MenuItem = { label: string; desc: string; href: string; icon: Component<{ size?: number }> };
export type Menu = {
  label: string;
  feature: { title: string; desc: string; cta: string; href: string };
  items: MenuItem[];
};

// The header menus. Each has one feature tile, then its links.
export const menus: Menu[] = [
  {
    label: "Privacy",
    feature: {
      title: "The whole list",
      desc: "Everything that can leave your laptop, and when.",
      cta: "See the list",
      href: "#privacy",
    },
    items: [
      { label: "Stays here", desc: "Chats, images, voice", href: "#privacy", icon: LockKeyholeIcon },
      { label: "Only on request", desc: "Downloads you start", href: "#privacy", icon: GlobalIcon },
      { label: "Logged", desc: "Every request, written down", href: "#privacy", icon: ClipboardIcon },
      { label: "Open code", desc: "Read it on GitHub", href: REPO, icon: ShieldCheckIcon },
    ],
  },
  {
    label: "Models",
    feature: {
      title: "Ready in the box",
      desc: "Starter models that run on a laptop.",
      cta: "See what ships",
      href: "#models",
    },
    items: [
      { label: "Starter models", desc: "Small and fast", href: "#models", icon: CpuIcon },
      { label: "Any GGUF", desc: "From Hugging Face", href: "#models", icon: DownloadMinimalisticIcon },
      { label: "Vision models", desc: "Read pictures too", href: "#models", icon: EyeIcon },
      { label: "More later", desc: "Video and others", href: "#models", icon: MagicWandIcon },
    ],
  },
];

export const navLinks = [
  { label: "See it work", href: "#jobs" },
  { label: "FAQ", href: "#faq" },
];

export const footerColumns = [
  {
    title: "Product",
    links: [
      { label: "See it work", href: "#jobs" },
      { label: "Privacy", href: "#privacy" },
      { label: "In the box", href: "#models" },
      { label: "Download", href: "#download" },
    ],
  },
  {
    title: "Project",
    links: [
      { label: "GitHub", href: REPO, external: true },
      { label: "Releases", href: RELEASES, external: true },
      { label: "Report a problem", href: `${REPO}/issues`, external: true },
    ],
  },
  {
    title: "Built on",
    links: [
      { label: "Handy (MIT)", href: "https://github.com/cjpais/Handy", external: true },
      { label: "llama.cpp", href: "https://github.com/ggml-org/llama.cpp", external: true },
      { label: "stable-diffusion.cpp", href: "https://github.com/leejet/stable-diffusion.cpp", external: true },
      { label: "sherpa-onnx", href: "https://github.com/k2-fsa/sherpa-onnx", external: true },
    ],
  },
];
