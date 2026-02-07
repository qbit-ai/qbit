import { render, screen, fireEvent } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import type { AdvancedSettings as AdvancedSettingsType, PrivacySettings, ProxySettings } from "@/lib/settings";
import { AdvancedSettings } from "./AdvancedSettings";

// Mock @tauri-apps/api/app
vi.mock("@tauri-apps/api/app", () => ({
  getVersion: vi.fn().mockResolvedValue("1.0.0"),
}));

const defaultAdvancedSettings: AdvancedSettingsType = {
  enable_experimental: false,
  log_level: "info",
  enable_llm_api_logs: false,
  extract_raw_sse: false,
};

const defaultPrivacy: PrivacySettings = {
  usage_statistics: false,
  log_prompts: false,
};

const defaultProxy: ProxySettings = {
  url: null,
  username: null,
  password: null,
  no_proxy: null,
  ca_cert_path: null,
  accept_invalid_certs: false,
};

describe("AdvancedSettings — Proxy section", () => {
  it("renders proxy fields", () => {
    render(
      <AdvancedSettings
        settings={defaultAdvancedSettings}
        privacy={defaultPrivacy}
        proxy={defaultProxy}
        onChange={vi.fn()}
        onPrivacyChange={vi.fn()}
        onProxyChange={vi.fn()}
      />
    );

    expect(screen.getByLabelText(/proxy url/i)).toBeInTheDocument();
    expect(screen.getByLabelText(/^username$/i)).toBeInTheDocument();
    expect(screen.getByLabelText(/^password$/i)).toBeInTheDocument();
    expect(screen.getByLabelText(/no proxy/i)).toBeInTheDocument();
    expect(screen.getByLabelText(/ca certificate/i)).toBeInTheDocument();
    expect(screen.getByLabelText(/accept invalid certificates/i)).toBeInTheDocument();
  });

  it("displays existing proxy values", () => {
    const proxy: ProxySettings = {
      url: "http://proxy:8080",
      username: "admin",
      password: "secret",
      no_proxy: "localhost,127.0.0.1",
      ca_cert_path: "/etc/ssl/certs/ca.pem",
      accept_invalid_certs: true,
    };

    render(
      <AdvancedSettings
        settings={defaultAdvancedSettings}
        privacy={defaultPrivacy}
        proxy={proxy}
        onChange={vi.fn()}
        onPrivacyChange={vi.fn()}
        onProxyChange={vi.fn()}
      />
    );

    expect(screen.getByLabelText(/proxy url/i)).toHaveValue("http://proxy:8080");
    expect(screen.getByLabelText(/^username$/i)).toHaveValue("admin");
    expect(screen.getByLabelText(/^password$/i)).toHaveValue("secret");
    expect(screen.getByLabelText(/no proxy/i)).toHaveValue("localhost,127.0.0.1");
    expect(screen.getByLabelText(/ca certificate/i)).toHaveValue("/etc/ssl/certs/ca.pem");
    expect(screen.getByLabelText(/accept invalid certificates/i)).toBeChecked();
  });

  it("calls onProxyChange when proxy URL is updated", () => {
    const onProxyChange = vi.fn();

    render(
      <AdvancedSettings
        settings={defaultAdvancedSettings}
        privacy={defaultPrivacy}
        proxy={defaultProxy}
        onChange={vi.fn()}
        onPrivacyChange={vi.fn()}
        onProxyChange={onProxyChange}
      />
    );

    const urlInput = screen.getByLabelText(/proxy url/i);
    fireEvent.change(urlInput, { target: { value: "http://newproxy:3128" } });

    expect(onProxyChange).toHaveBeenCalledWith({
      ...defaultProxy,
      url: "http://newproxy:3128",
    });
  });

  it("sends null for empty proxy fields", () => {
    const onProxyChange = vi.fn();
    const proxy: ProxySettings = {
      url: "http://proxy:8080",
      username: null,
      password: null,
      no_proxy: null,
    };

    render(
      <AdvancedSettings
        settings={defaultAdvancedSettings}
        privacy={defaultPrivacy}
        proxy={proxy}
        onChange={vi.fn()}
        onPrivacyChange={vi.fn()}
        onProxyChange={onProxyChange}
      />
    );

    const urlInput = screen.getByLabelText(/proxy url/i);
    fireEvent.change(urlInput, { target: { value: "" } });

    expect(onProxyChange).toHaveBeenCalledWith({
      ...proxy,
      url: null,
    });
  });

  it("password field is masked", () => {
    render(
      <AdvancedSettings
        settings={defaultAdvancedSettings}
        privacy={defaultPrivacy}
        proxy={defaultProxy}
        onChange={vi.fn()}
        onPrivacyChange={vi.fn()}
        onProxyChange={vi.fn()}
      />
    );

    const passwordInput = screen.getByLabelText(/^password$/i);
    expect(passwordInput).toHaveAttribute("type", "password");
  });

  it("renders CA cert and accept_invalid_certs fields", () => {
    render(
      <AdvancedSettings
        settings={defaultAdvancedSettings}
        privacy={defaultPrivacy}
        proxy={defaultProxy}
        onChange={vi.fn()}
        onPrivacyChange={vi.fn()}
        onProxyChange={vi.fn()}
      />
    );

    const caCertInput = screen.getByLabelText(/ca certificate/i);
    expect(caCertInput).toBeInTheDocument();
    expect(caCertInput).toHaveAttribute("type", "text");

    const acceptInvalidSwitch = screen.getByLabelText(/accept invalid certificates/i);
    expect(acceptInvalidSwitch).toBeInTheDocument();
    expect(acceptInvalidSwitch).not.toBeChecked();
  });

  it("accept_invalid_certs switch calls onProxyChange", () => {
    const onProxyChange = vi.fn();

    render(
      <AdvancedSettings
        settings={defaultAdvancedSettings}
        privacy={defaultPrivacy}
        proxy={defaultProxy}
        onChange={vi.fn()}
        onPrivacyChange={vi.fn()}
        onProxyChange={onProxyChange}
      />
    );

    const acceptInvalidSwitch = screen.getByLabelText(/accept invalid certificates/i);
    fireEvent.click(acceptInvalidSwitch);

    expect(onProxyChange).toHaveBeenCalledWith({
      ...defaultProxy,
      accept_invalid_certs: true,
    });
  });
});
