import { Check, Copy } from "lucide-react";
import { useCopyToClipboard } from "@/hooks/useCopyToClipboard";
import { cn } from "@/lib/utils";

interface CopyButtonProps {
  content: string;
  className?: string;
  "data-testid"?: string;
}

export function CopyButton({ content, className, "data-testid": testId }: CopyButtonProps) {
  const { copied, copy } = useCopyToClipboard();

  const handleCopy = async () => {
    await copy(content);
  };

  return (
    <button
      type="button"
      onClick={handleCopy}
      className={cn(
        "p-1.5 rounded transition-all hover:bg-muted text-muted-foreground hover:text-foreground",
        copied && "text-[var(--color-success)]",
        className
      )}
      title={copied ? "Copied!" : "Copy code"}
      data-testid={testId}
    >
      {copied ? <Check className="w-4 h-4" /> : <Copy className="w-4 h-4" />}
    </button>
  );
}
