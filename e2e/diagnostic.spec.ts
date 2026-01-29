import { expect, test } from "@playwright/test";
import { waitForAppReady } from "./helpers/app";

/**
 * Diagnostic test to understand the 34 textareas issue
 */
test.describe("Diagnostic Tests", () => {
  test("track mount/unmount console logs", async ({ page }) => {
    // Capture console messages
    const unifiedInputLogs: string[] = [];
    const paneLeafLogs: string[] = [];
    const errorLogs: string[] = [];

    page.on("console", (msg) => {
      const text = msg.text();
      const type = msg.type();

      if (text.includes("[UnifiedInput]")) {
        unifiedInputLogs.push(text);
      }
      if (text.includes("[PaneLeaf]")) {
        paneLeafLogs.push(text);
      }
      if (type === "error") {
        errorLogs.push(text);
      }
    });

    await page.goto("/");
    await page.waitForLoadState("domcontentloaded");

    // Wait for mock mode
    await page.waitForFunction(
      () =>
        (window as unknown as { __MOCK_BROWSER_MODE__?: boolean }).__MOCK_BROWSER_MODE__ === true,
      { timeout: 15000 }
    );

    // Wait for the app to stabilize and collect logs
    await page.waitForTimeout(3000);

    console.log("=== PaneLeaf mount/unmount logs ===");
    console.log(`Total: ${paneLeafLogs.length}`);
    for (const log of paneLeafLogs.slice(0, 10)) {
      console.log(`  ${log}`);
    }
    const paneLeafMounts = paneLeafLogs.filter((l) => l.includes("MOUNT")).length;
    const paneLeafUnmounts = paneLeafLogs.filter((l) => l.includes("UNMOUNT")).length;
    console.log(`  PaneLeaf mounts: ${paneLeafMounts}, unmounts: ${paneLeafUnmounts}`);

    console.log("\n=== UnifiedInput mount/unmount logs ===");
    console.log(`Total: ${unifiedInputLogs.length}`);
    for (const log of unifiedInputLogs.slice(0, 10)) {
      console.log(`  ${log}`);
    }
    const mountCount = unifiedInputLogs.filter((l) => l.includes("MOUNT")).length;
    const unmountCount = unifiedInputLogs.filter((l) => l.includes("UNMOUNT")).length;
    console.log(`  UnifiedInput mounts: ${mountCount}, unmounts: ${unmountCount}`);

    console.log("\n=== Console Errors ===");
    console.log(`Total errors: ${errorLogs.length}`);
    for (const log of errorLogs.slice(0, 10)) {
      console.log(`  ${log.slice(0, 200)}`);
    }

    // In a stable app, there should be very few mount/unmount cycles
    expect(mountCount).toBeLessThan(10);
  });

  test("track requestAnimationFrame calls", async ({ page }) => {
    await waitForAppReady(page);

    // Track RAF calls with stack traces to identify sources
    const rafTracking = await page.evaluate(async () => {
      return new Promise<{
        rafCalls: number;
        setTimeoutCalls: number;
        stackSamples: string[];
        sourceCounts: Record<string, number>;
      }>((resolve) => {
        const results = {
          rafCalls: 0,
          setTimeoutCalls: 0,
          stackSamples: [] as string[],
          sourceCounts: {} as Record<string, number>,
        };

        // Patch RAF with stack trace sampling
        const origRAF = window.requestAnimationFrame;
        window.requestAnimationFrame = (callback) => {
          results.rafCalls++;

          // Sample stack traces (first 20, then every 100th)
          if (results.rafCalls <= 20 || results.rafCalls % 100 === 0) {
            try {
              const stack = new Error().stack || "";
              const lines = stack.split("\n").slice(2, 5);
              const source = lines.join(" <- ").replace(/\s+/g, " ").slice(0, 200);
              if (results.stackSamples.length < 10) {
                results.stackSamples.push(source);
              }

              // Count by first function name
              const match = lines[0]?.match(/at\s+(\S+)/);
              const funcName = match?.[1] || "unknown";
              results.sourceCounts[funcName] = (results.sourceCounts[funcName] || 0) + 1;
            } catch {
              // Ignore stack trace errors
            }
          }

          return origRAF.call(window, callback);
        };

        // Patch setTimeout (count only short timeouts)
        const origSetTimeout = window.setTimeout;
        (window as unknown as { setTimeout: typeof setTimeout }).setTimeout = (
          callback: TimerHandler,
          delay?: number,
          ...args: unknown[]
        ) => {
          if (typeof delay === "number" && delay < 100) {
            results.setTimeoutCalls++;
          }
          return origSetTimeout.call(window, callback, delay, ...args);
        };

        // Wait 2 seconds
        origSetTimeout(() => {
          // Restore originals
          window.requestAnimationFrame = origRAF;
          (window as unknown as { setTimeout: typeof setTimeout }).setTimeout = origSetTimeout;
          resolve(results);
        }, 2000);
      });
    });

    console.log("Animation/timer tracking over 2 seconds:");
    console.log(`  requestAnimationFrame calls: ${rafTracking.rafCalls}`);
    console.log(`  Short setTimeout calls (<100ms): ${rafTracking.setTimeoutCalls}`);
    console.log(`  RAF source counts: ${JSON.stringify(rafTracking.sourceCounts, null, 2)}`);
    console.log(
      `  Stack samples: ${JSON.stringify(rafTracking.stackSamples.slice(0, 5), null, 2)}`
    );

    // If there are excessive RAF calls, something is looping
    // 60fps would be ~120 calls in 2 seconds, allow some margin
    expect(rafTracking.rafCalls).toBeLessThan(500);
  });

  test("track store updates", async ({ page }) => {
    await waitForAppReady(page);

    // Track how many times the store updates
    const storeUpdates = await page.evaluate(async () => {
      return new Promise<{
        updateCount: number;
        updateSources: string[];
      }>((resolve) => {
        const results = {
          updateCount: 0,
          updateSources: [] as string[],
        };

        // Get the store and subscribe to changes
        const store = (
          window as unknown as {
            __QBIT_STORE__?: { subscribe: (fn: () => void) => () => void; getState: () => unknown };
          }
        ).__QBIT_STORE__;
        if (!store) {
          resolve(results);
          return;
        }

        let lastState = JSON.stringify(store.getState());
        const unsubscribe = store.subscribe(() => {
          results.updateCount++;
          const newState = JSON.stringify(store.getState());
          if (results.updateSources.length < 10) {
            // Try to figure out what changed
            try {
              const oldObj = JSON.parse(lastState);
              const newObj = JSON.parse(newState);
              for (const key of Object.keys(newObj)) {
                if (JSON.stringify(oldObj[key]) !== JSON.stringify(newObj[key])) {
                  results.updateSources.push(key);
                  break;
                }
              }
            } catch {
              results.updateSources.push("parse-error");
            }
          }
          lastState = newState;
        });

        // Wait 2 seconds
        setTimeout(() => {
          unsubscribe();
          resolve(results);
        }, 2000);
      });
    });

    console.log("Store update tracking over 2 seconds:");
    console.log(`  Total updates: ${storeUpdates.updateCount}`);
    console.log(`  Update sources (first 10): ${storeUpdates.updateSources.join(", ")}`);

    // If there are continuous store updates, something is wrong
    expect(storeUpdates.updateCount).toBeLessThan(10);
  });

  test("track DOM mutations with MutationObserver", async ({ page }) => {
    await waitForAppReady(page);

    // Set up MutationObserver to track only unified-input related mutations
    const mutations = await page.evaluate(async () => {
      return new Promise<{
        total: number;
        childListCount: number;
        attributeCount: number;
        unifiedInputAdditions: number;
        unifiedInputRemovals: number;
        xtermAdditions: number;
        xtermRemovals: number;
        samples: string[];
        attrMutationTargets: Record<string, number>;
        attrNames: Record<string, number>;
      }>((resolve) => {
        const results = {
          total: 0,
          childListCount: 0,
          attributeCount: 0,
          unifiedInputAdditions: 0,
          unifiedInputRemovals: 0,
          xtermAdditions: 0,
          xtermRemovals: 0,
          samples: [] as string[],
          attrMutationTargets: {} as Record<string, number>,
          attrNames: {} as Record<string, number>,
        };

        // Helper to check if an element contains unified-input
        const hasUnifiedInput = (el: Element): boolean => {
          return (
            el.querySelector?.('[data-testid="unified-input"]') !== null ||
            (el instanceof HTMLTextAreaElement &&
              el.getAttribute("data-testid") === "unified-input")
          );
        };

        // Helper to check if an element contains xterm textarea
        const hasXtermTextarea = (el: Element): boolean => {
          return (
            el.querySelector?.(".xterm-helper-textarea") !== null ||
            el.classList?.contains("xterm-helper-textarea")
          );
        };

        const observer = new MutationObserver((mutations) => {
          for (const mutation of mutations) {
            results.total++;
            if (mutation.type === "childList") {
              results.childListCount++;
              // Check for additions
              for (const node of Array.from(mutation.addedNodes)) {
                if (node instanceof Element) {
                  if (hasUnifiedInput(node)) {
                    results.unifiedInputAdditions++;
                    if (results.samples.length < 10) {
                      results.samples.push(
                        `ADD unified-input: ${node.tagName} to ${mutation.target instanceof Element ? mutation.target.className.slice(0, 30) : "unknown"}`
                      );
                    }
                  } else if (hasXtermTextarea(node)) {
                    results.xtermAdditions++;
                  }
                }
              }
              // Check for removals
              for (const node of Array.from(mutation.removedNodes)) {
                if (node instanceof Element) {
                  if (hasUnifiedInput(node)) {
                    results.unifiedInputRemovals++;
                    if (results.samples.length < 10) {
                      results.samples.push(
                        `REMOVE unified-input: ${node.tagName} from ${mutation.target instanceof Element ? mutation.target.className.slice(0, 30) : "unknown"}`
                      );
                    }
                  } else if (hasXtermTextarea(node)) {
                    results.xtermRemovals++;
                  }
                }
              }
            } else if (mutation.type === "attributes") {
              results.attributeCount++;
              // Track which elements are getting attribute mutations
              const target = mutation.target as Element;
              const targetKey = `${target.tagName}${target.className ? `.${target.className.split(" ")[0]}` : ""}`;
              results.attrMutationTargets[targetKey] =
                (results.attrMutationTargets[targetKey] || 0) + 1;
              // Track which attributes are being mutated
              const attrName = mutation.attributeName || "unknown";
              results.attrNames[attrName] = (results.attrNames[attrName] || 0) + 1;
            }
          }
        });

        observer.observe(document.body, {
          childList: true,
          subtree: true,
          attributes: true,
          attributeOldValue: true,
        });

        // Wait 2 seconds
        setTimeout(() => {
          observer.disconnect();
          resolve(results);
        }, 2000);
      });
    });

    console.log("DOM Mutation results over 2 seconds:");
    console.log(`  Total mutations: ${mutations.total}`);
    console.log(`  Child list mutations: ${mutations.childListCount}`);
    console.log(`  Attribute mutations: ${mutations.attributeCount}`);
    console.log(`  UnifiedInput additions: ${mutations.unifiedInputAdditions}`);
    console.log(`  UnifiedInput removals: ${mutations.unifiedInputRemovals}`);
    console.log(`  Xterm additions: ${mutations.xtermAdditions}`);
    console.log(`  Xterm removals: ${mutations.xtermRemovals}`);
    console.log(`  Attribute mutation targets: ${JSON.stringify(mutations.attrMutationTargets)}`);
    console.log(`  Attribute names: ${JSON.stringify(mutations.attrNames)}`);
    console.log(`  Samples: ${JSON.stringify(mutations.samples, null, 2)}`);

    // If there are continuous mutations, something is wrong
    // In a stable app, there should be very few mutations when idle
    expect(mutations.unifiedInputAdditions).toBeLessThan(5);
    expect(mutations.unifiedInputRemovals).toBeLessThan(5);
  });

  test("check render loop by tracking React keys", async ({ page }) => {
    await waitForAppReady(page);

    // Track if React is continuously re-keying elements
    const keyChanges = await page.evaluate(async () => {
      return new Promise<{
        uniqueKeys: Set<string>;
        keyChangeCount: number;
        elementReplaceCount: number;
      }>((resolve) => {
        const results = {
          uniqueKeys: new Set<string>(),
          keyChangeCount: 0,
          elementReplaceCount: 0,
        };

        let lastTextarea: HTMLTextAreaElement | null = null;
        const interval = setInterval(() => {
          const textarea = document.querySelector(
            '[data-testid="unified-input"]'
          ) as HTMLTextAreaElement;
          if (textarea) {
            // Check for React key
            const reactKey = (
              textarea as unknown as {
                _reactRootContainer?: unknown;
                __reactFiber$?: { key?: string };
              }
            ).__reactFiber$?.key;
            if (reactKey) {
              results.uniqueKeys.add(reactKey);
              results.keyChangeCount++;
            }
            // Check if element reference changed
            if (lastTextarea && textarea !== lastTextarea) {
              results.elementReplaceCount++;
            }
            lastTextarea = textarea;
          }
        }, 100);

        setTimeout(() => {
          clearInterval(interval);
          resolve({
            uniqueKeys: results.uniqueKeys,
            keyChangeCount: results.uniqueKeys.size,
            elementReplaceCount: results.elementReplaceCount,
          });
        }, 2000);
      });
    });

    console.log("React key tracking over 2 seconds:");
    console.log(`  Unique keys seen: ${keyChanges.keyChangeCount}`);
    console.log(`  Element replacements: ${keyChanges.elementReplaceCount}`);

    // If elements are being replaced continuously, that's a problem
    expect(keyChanges.elementReplaceCount).toBeLessThan(5);
  });

  test("identify parent re-renders", async ({ page }) => {
    await waitForAppReady(page);

    // Track which parent elements are changing
    const parentChanges = await page.evaluate(async () => {
      return new Promise<{
        parentPath: string[];
        parentChanges: number;
        textareaChanges: number;
      }>((resolve) => {
        const results = {
          parentPath: [] as string[],
          parentChanges: 0,
          textareaChanges: 0,
        };

        let lastTextarea: HTMLTextAreaElement | null = null;
        let lastParentSignature = "";

        const interval = setInterval(() => {
          const textarea = document.querySelector(
            '[data-testid="unified-input"]'
          ) as HTMLTextAreaElement;
          if (textarea) {
            // Get parent path
            const parentPath: string[] = [];
            let el: HTMLElement | null = textarea.parentElement;
            while (el && parentPath.length < 10) {
              const id = el.id ? `#${el.id}` : "";
              const classes = el.className
                ? `.${el.className.split(" ").slice(0, 2).join(".")}`
                : "";
              parentPath.push(`${el.tagName}${id}${classes}`);
              el = el.parentElement;
            }

            const signature = parentPath.join(" > ");
            if (lastParentSignature && signature !== lastParentSignature) {
              results.parentChanges++;
            }
            lastParentSignature = signature;
            results.parentPath = parentPath;

            if (lastTextarea && textarea !== lastTextarea) {
              results.textareaChanges++;
            }
            lastTextarea = textarea;
          }
        }, 100);

        setTimeout(() => {
          clearInterval(interval);
          resolve(results);
        }, 2000);
      });
    });

    console.log("Parent tracking over 2 seconds:");
    console.log(`  Parent path: ${parentChanges.parentPath.slice(0, 5).join(" > ")}`);
    console.log(`  Parent structure changes: ${parentChanges.parentChanges}`);
    console.log(`  Textarea element changes: ${parentChanges.textareaChanges}`);

    expect(parentChanges.textareaChanges).toBeLessThan(5);
  });

  test("count elements and check store state", async ({ page }) => {
    await waitForAppReady(page);

    // Get the number of textareas in the DOM
    const textareaCount = await page.locator("textarea").count();
    console.log(`Total textareas in DOM: ${textareaCount}`);

    // Get details about ALL textareas to understand their source
    const allTextareaDetails = await page.evaluate(() => {
      const textareas = document.querySelectorAll("textarea");
      return Array.from(textareas)
        .slice(0, 10)
        .map((ta) => {
          const testId = ta.getAttribute("data-testid");
          const classes = ta.className;
          const parent = ta.parentElement;
          const parentClasses = parent?.className || "";
          return {
            testId,
            classes: classes.slice(0, 100),
            parentClasses: parentClasses.slice(0, 100),
            placeholder: ta.placeholder?.slice(0, 50),
            id: ta.id,
            name: ta.name,
          };
        });
    });
    console.log("First 10 textareas details:", JSON.stringify(allTextareaDetails, null, 2));

    // Get unified-input textareas specifically
    const unifiedInputCount = await page.locator('[data-testid="unified-input"]').count();
    console.log(`UnifiedInput textareas: ${unifiedInputCount}`);

    // Check visibility of each textarea
    const visibilityData = await page.evaluate(() => {
      const textareas = document.querySelectorAll('[data-testid="unified-input"]');
      return Array.from(textareas).map((ta, i) => {
        const rect = ta.getBoundingClientRect();
        const style = window.getComputedStyle(ta);
        return {
          index: i,
          width: rect.width,
          height: rect.height,
          display: style.display,
          visibility: style.visibility,
          opacity: style.opacity,
          inViewport:
            rect.top >= 0 &&
            rect.left >= 0 &&
            rect.bottom <= window.innerHeight &&
            rect.right <= window.innerWidth,
        };
      });
    });
    console.log("Textarea visibility data:", JSON.stringify(visibilityData, null, 2));

    // Get store state
    const storeState = await page.evaluate(() => {
      const store = (window as unknown as { __QBIT_STORE__?: { getState: () => unknown } })
        .__QBIT_STORE__;
      if (!store) return null;
      const state = store.getState() as {
        sessions: Record<string, { id: string; tabType?: string; name: string }>;
        tabLayouts: Record<string, { root: unknown; focusedPaneId: string }>;
        activeSessionId: string | null;
        homeTabId: string | null;
      };
      return {
        sessionCount: Object.keys(state.sessions).length,
        sessionIds: Object.keys(state.sessions),
        sessions: Object.entries(state.sessions).map(([id, s]) => ({
          id,
          tabType: s.tabType,
          name: s.name,
        })),
        tabLayoutCount: Object.keys(state.tabLayouts).length,
        tabLayoutIds: Object.keys(state.tabLayouts),
        activeSessionId: state.activeSessionId,
        homeTabId: state.homeTabId,
      };
    });
    console.log("Store state:", JSON.stringify(storeState, null, 2));

    // Count pane sections
    const paneCount = await page.locator("section[aria-label*='Pane']").count();
    console.log(`Pane sections: ${paneCount}`);

    // Expectations based on normal initialization:
    // - 2 sessions (Home + Terminal)
    // - 2 tab layouts
    // - 1 UnifiedInput (only Terminal tab renders it, Home tab renders HomeView)
    expect(storeState?.sessionCount).toBeLessThanOrEqual(3); // Allow for some variation
    expect(unifiedInputCount).toBeLessThanOrEqual(5); // Should be 1-2, but allow for render cycles

    // If we have too many textareas, log the DOM structure
    if (unifiedInputCount > 5) {
      const domStructure = await page.evaluate(() => {
        const textareas = document.querySelectorAll('[data-testid="unified-input"]');
        return Array.from(textareas).map((ta) => {
          const parents: string[] = [];
          let el = ta.parentElement;
          while (el && parents.length < 5) {
            parents.push(
              `${el.tagName}${el.id ? `#${el.id}` : ""}${el.className ? `.${el.className.split(" ").slice(0, 2).join(".")}` : ""}`
            );
            el = el.parentElement;
          }
          return parents;
        });
      });
      console.log("DOM structure of textareas:", JSON.stringify(domStructure, null, 2));
    }
  });

  test("check textarea stability", async ({ page }) => {
    await waitForAppReady(page);

    // Count how many times the textarea reference changes over time
    const changes: { time: number; id: string | null }[] = [];
    let lastId: string | null = null;

    for (let i = 0; i < 20; i++) {
      const currentId = await page.evaluate(() => {
        const textarea = document.querySelector('[data-testid="unified-input"]');
        if (!textarea) return null;
        // Create a unique identifier based on the element's position
        const rect = textarea.getBoundingClientRect();
        return `${rect.x}-${rect.y}-${rect.width}-${rect.height}`;
      });

      if (currentId !== lastId) {
        changes.push({ time: Date.now(), id: currentId });
        lastId = currentId;
      }
      await page.waitForTimeout(100);
    }

    console.log(`Textarea reference changes: ${changes.length} changes over 2 seconds`);
    console.log("Changes:", JSON.stringify(changes, null, 2));

    // The textarea should be stable - at most 1-2 changes during initial render
    expect(changes.length).toBeLessThanOrEqual(3);
  });

  test("check for React StrictMode effects", async ({ page }) => {
    // This test checks if StrictMode is causing issues
    await page.goto("/");
    await page.waitForLoadState("domcontentloaded");

    // Wait for mock mode flag
    await page.waitForFunction(
      () =>
        (window as unknown as { __MOCK_BROWSER_MODE__?: boolean }).__MOCK_BROWSER_MODE__ === true,
      { timeout: 15000 }
    );

    // Wait a bit for React to settle
    await page.waitForTimeout(2000);

    // Count textareas at different intervals
    const counts: number[] = [];
    for (let i = 0; i < 5; i++) {
      const count = await page.locator('[data-testid="unified-input"]').count();
      counts.push(count);
      await page.waitForTimeout(500);
    }
    console.log("Textarea counts over time:", counts);

    // Check if count is stable
    const stable = counts.every((c) => c === counts[0]);
    console.log(`Textarea count stable: ${stable}, values: ${counts.join(", ")}`);

    // The count should stabilize
    expect(counts[counts.length - 1]).toBeLessThanOrEqual(5);
  });
});
