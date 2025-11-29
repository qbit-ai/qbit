export interface TruncationResult {
  truncatedContent: string;
  isTruncated: boolean;
  totalLines: number;
  hiddenLines: number;
}

export function truncateByLines(content: string, maxLines: number = 10): TruncationResult {
  const lines = content.split("\n");
  const totalLines = lines.length;
  const isTruncated = totalLines > maxLines;

  return {
    truncatedContent: isTruncated ? lines.slice(0, maxLines).join("\n") : content,
    isTruncated,
    totalLines,
    hiddenLines: isTruncated ? totalLines - maxLines : 0,
  };
}
