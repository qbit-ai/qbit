/**
 * Project configuration API.
 *
 * Projects are stored as individual TOML files in ~/.qbit/projects/.
 */

import { invoke } from "@tauri-apps/api/core";

/** Project form data matching the SetupProjectModal form. */
export interface ProjectFormData {
  name: string;
  rootPath: string;
  worktreesDir: string;
  testCommand: string;
  lintCommand: string;
  buildCommand: string;
  startCommand: string;
  worktreeInitScript: string;
}

/** Project data returned from the backend. */
export interface ProjectData {
  name: string;
  rootPath: string;
  worktreesDir: string | null;
  testCommand: string | null;
  lintCommand: string | null;
  buildCommand: string | null;
  startCommand: string | null;
  worktreeInitScript: string | null;
}

/**
 * Save a new or updated project configuration.
 */
export async function saveProject(form: ProjectFormData): Promise<void> {
  await invoke("save_project", { form });
}

/**
 * Delete a project configuration by name.
 * @returns true if the project was deleted, false if it didn't exist.
 */
export async function deleteProject(name: string): Promise<boolean> {
  return invoke<boolean>("delete_project_config", { name });
}

/**
 * List all saved project configurations.
 */
export async function listProjectConfigs(): Promise<ProjectData[]> {
  return invoke<ProjectData[]>("list_project_configs");
}

/**
 * Get a single project configuration by name.
 * @returns The project config or null if not found.
 */
export async function getProjectConfig(name: string): Promise<ProjectData | null> {
  return invoke<ProjectData | null>("get_project_config", { name });
}
