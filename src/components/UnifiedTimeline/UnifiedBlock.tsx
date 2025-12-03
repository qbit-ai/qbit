import { memo } from "react";
import { AgentMessage } from "@/components/AgentChat/AgentMessage";
import { CommandBlock } from "@/components/CommandBlock/CommandBlock";
import type { UnifiedBlock as UnifiedBlockType } from "@/store";
import { useStore } from "@/store";

interface UnifiedBlockProps {
  block: UnifiedBlockType;
}

export const UnifiedBlock = memo(function UnifiedBlock({ block }: UnifiedBlockProps) {
  const toggleBlockCollapse = useStore((state) => state.toggleBlockCollapse);

  switch (block.type) {
    case "command":
      return <CommandBlock block={block.data} onToggleCollapse={toggleBlockCollapse} />;

    case "agent_message":
      return <AgentMessage message={block.data} />;

    case "agent_streaming":
      // This shouldn't appear in the timeline as streaming is handled separately
      // but we include it for completeness
      return null;

    default:
      return null;
  }
});
