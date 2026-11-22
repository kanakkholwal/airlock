import {
  ChatLineIcon,
  ClipboardIcon,
  CpuIcon,
  DocumentTextIcon,
  DownloadMinimalisticIcon,
  EyeIcon,
  FolderOpenIcon,
  GalleryIcon,
  GlobalIcon,
  KeyboardIcon,
  LockKeyholeIcon,
  MagicWandIcon,
  ShieldCheckIcon,
  VolumeLoudIcon,
} from "@solar-icons/svelte/linear";
import type { Component } from "svelte";

// Decision (product): the page reads from this data. A new tool, question, picture or link is a
// line here, and the sections render whatever is listed.
export const RELEASES = "https://github.com/kanakkholwal/airlock/releases";
export const REPO = "https://github.com/kanakkholwal/airlock";

export const tools = [
  { id: "chat", name: "Chat", role: "Local models", description: "Ask, write and reason with a model on your own machine. Pictures can go in too." },
  { id: "dictation", name: "Dictation", role: "Any app", description: "Hold a shortcut anywhere, speak, and the words are typed where your cursor is." },
  { id: "transcribe", name: "Transcribe", role: "Files", description: "Turn recordings into text with timestamps, then export SRT or VTT." },
  { id: "voice", name: "Read aloud", role: "Natural voices", description: "Paste any text and hear it in a clear voice that runs on this computer." },
  { id: "images", name: "Images", role: "Your GPU", description: "Describe a picture and make it on your own graphics card. It is saved to your private library." },
  { id: "library", name: "Library", role: "Private by default", description: "Everything you make stays in one place, and nothing is shared unless you say so." },
];

// What can leave the computer, taken from Settings, Privacy in the app.
export const ledger = [
  { what: "Chats, images and voice", where: "This computer", when: "Never leaves" },
  { what: "Model search", where: "Hugging Face", when: "When you search. Your words are sent" },
  { what: "Model downloads", where: "Hugging Face", when: "When you press Download" },
  { what: "Engine downloads", where: "GitHub", when: "When you press Install" },
  { what: "Update checks", where: "GitHub", when: "Version number only" },
];

export const faq = [
  {
    question: "Is it really offline?",
    answer:
      "Chat, dictation, voice and images run on your computer. Airlock only goes online when you search for or download a model or an engine, or when it checks for an update. Settings, then Privacy, lists every case.",
  },
  {
    question: "What computer do I need?",
    answer:
      "A Windows PC for now. A graphics card makes chat and images much faster, and everything also runs on the processor, only slower. Small models run on a 4 GB graphics card.",
  },
  {
    question: "Is it free?",
    answer: "Yes, and the code is open. Each model has its own license, shown before you download it.",
  },
  {
    question: "Where is my data kept?",
    answer: "In a folder on your computer: chats, recordings, pictures and settings. Nothing is uploaded.",
  },
  {
    question: "Why does Windows warn me when I open it?",
    answer: "This pre-release is not code-signed yet, so SmartScreen asks you to confirm before it runs.",
  },
  {
    question: "Can it use cloud models?",
    answer: "Not today. If that ever changes, nothing will leave your computer without your say, and each request will be logged.",
  },
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
      title: "What can leave",
      desc: "The whole list, with when and where to.",
      cta: "Read the ledger",
      href: "#privacy",
    },
    items: [
      { label: "Stays here", desc: "Chats, images and voice", href: "#privacy", icon: LockKeyholeIcon },
      { label: "Only on request", desc: "Downloads when you press a button", href: "#privacy", icon: GlobalIcon },
      { label: "Logged", desc: "Every outgoing request is written down", href: "#privacy", icon: ClipboardIcon },
      { label: "Open code", desc: "Check it yourself on GitHub", href: REPO, icon: ShieldCheckIcon },
    ],
  },
  {
    label: "Models",
    feature: {
      title: "Start small, add more",
      desc: "A short list that fits modest hardware, plus any GGUF from Hugging Face.",
      cta: "See the models",
      href: "#models",
    },
    items: [
      { label: "Starter models", desc: "Small, fast, licensed", href: "#models", icon: CpuIcon },
      { label: "Any GGUF", desc: "From Hugging Face", href: "#models", icon: DownloadMinimalisticIcon },
      { label: "Vision models", desc: "Read pictures too", href: "#models", icon: EyeIcon },
      { label: "More engines", desc: "Video and others, later", href: "#models", icon: MagicWandIcon },
    ],
  },
];

export const footerColumns = [
  {
    title: "Product",
    links: [
      { label: "Privacy", href: "#privacy" },
      { label: "Models", href: "#models" },
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
