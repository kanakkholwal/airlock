// Decision (product): errors keep their real message so the error screen can show it,
// instead of the generic "Internal Error".
export const handleError = ({ error }: { error: unknown }) => ({
  message: error instanceof Error ? error.message : String(error),
});
