import type { NoticeCode } from "../../bindings";

type NoticeDisplay = {
  title: string;
  /** Without one, the live overlay shows the notice's own detail instead. */
  explanation?: () => string;
};

const os = () => {
  const ua = navigator.userAgent;
  if (ua.includes("Windows")) return "windows";
  if (ua.includes("Mac")) return "macos";
  return "linux";
};

const MIC_DENIED = {
  windows: "Enable microphone access in Settings → Privacy & security → Microphone (including desktop app access).",
  macos: "Grant microphone access in System Settings → Privacy & Security → Microphone.",
  linux: "Grant microphone access in your system's sound or privacy settings.",
} as const;

// Provenance: Handy src/lib/notices.ts, English only. Add a code here when Rust adds one.
const NOTICES: Record<NoticeCode, NoticeDisplay> = {
  TranscriptionFailed: { title: "Transcription Failed" },
  PasteFailed: {
    title: "Failed to Paste Text",
    explanation: () => "Text could not be pasted into the active application.",
  },
  PasteBlockedElevated: {
    title: "Can't Paste into Admin Apps",
    explanation: () =>
      "The active application is running as administrator, so Airlock can't paste into it. You can copy the transcript from History.",
  },
  NoMicrophone: {
    title: "No Microphone Found",
    explanation: () => "No audio input device was detected. Please connect a microphone or headset and try again.",
  },
  MicrophoneSilent: {
    title: "Microphone Is Silent",
    explanation: () =>
      "Your microphone sent only silence. Check that it isn't muted, or choose a different microphone in Settings.",
  },
  MicPermissionDenied: {
    title: "Microphone Access Denied",
    explanation: () => MIC_DENIED[os()],
  },
  RecordingFailed: { title: "Failed to Start Recording" },
  ModelLoadFailed: {
    title: "Failed to Load Model",
    explanation: () => "Airlock couldn't load this model. Try again or choose a different one.",
  },
  NoModelSelected: {
    title: "No Model Selected",
    explanation: () => "Download or select a model in Settings > Models.",
  },
  PostProcessFailed: {
    title: "Pasted Without Post-Processing",
    explanation: () => "Post Process didn't finish, so the transcript was pasted as is.",
  },
  FileTranscriptionFailed: { title: "File Transcription Failed" },
  FilePostProcessFailed: {
    title: "Transcribed Without Post-Processing",
    explanation: () => "Post Process didn't finish, so the transcript was saved as is.",
  },
};

export const noticeDisplay = (code: NoticeCode): NoticeDisplay => NOTICES[code];
