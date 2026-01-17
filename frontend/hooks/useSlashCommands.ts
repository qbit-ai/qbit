import { useCallback, useEffect, useState } from "react";
import { logger } from "@/lib/logger";
import { listPrompts, listSkills, type PromptInfo, type SkillInfo } from "@/lib/tauri";

export type SlashCommandType = "prompt" | "skill";

export interface SlashCommand {
  name: string;
  path: string;
  source: "global" | "local";
  type: SlashCommandType;
  description?: string;
  skill?: SkillInfo;
}

export function useSlashCommands(workingDirectory?: string) {
  const [commands, setCommands] = useState<SlashCommand[]>([]);
  const [prompts, setPrompts] = useState<PromptInfo[]>([]);
  const [isLoading, setIsLoading] = useState(false);

  const loadCommands = useCallback(async () => {
    setIsLoading(true);
    try {
      // Load skills and prompts in parallel
      const [skillsResult, promptsResult] = await Promise.all([
        listSkills(workingDirectory).catch((err) => {
          logger.error("Failed to load skills:", err);
          return [] as SkillInfo[];
        }),
        listPrompts(workingDirectory).catch((err) => {
          logger.error("Failed to load prompts:", err);
          return [] as PromptInfo[];
        }),
      ]);

      // Keep prompts for backward compatibility
      setPrompts(promptsResult);

      // Merge skills and prompts into unified commands
      // Prompts take precedence over skills with the same name
      const commandMap = new Map<string, SlashCommand>();

      // Add skills first (lower precedence)
      for (const skill of skillsResult) {
        commandMap.set(skill.name, {
          name: skill.name,
          path: skill.path,
          source: skill.source as "global" | "local",
          type: "skill",
          description: skill.description,
          skill,
        });
      }

      // Add prompts (higher precedence - overwrites skills with same name)
      for (const prompt of promptsResult) {
        commandMap.set(prompt.name, {
          name: prompt.name,
          path: prompt.path,
          source: prompt.source,
          type: "prompt",
        });
      }

      // Convert to sorted array
      const mergedCommands = Array.from(commandMap.values()).sort((a, b) =>
        a.name.toLowerCase().localeCompare(b.name.toLowerCase())
      );

      setCommands(mergedCommands);
    } finally {
      setIsLoading(false);
    }
  }, [workingDirectory]);

  // Load commands on mount and when working directory changes
  useEffect(() => {
    loadCommands();
  }, [loadCommands]);

  return { commands, prompts, isLoading, reload: loadCommands };
}
