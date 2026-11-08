import "@testing-library/jest-dom/vitest";

if (typeof window !== "undefined") {
  (window as any).__TAURI_EVENT_PLUGIN_INTERNALS__ = new Proxy(
    {},
    {
      get(target, prop) {
        if (prop === "unregisterListener") {
          return () => {};
        }
        return (target as any)[prop];
      },
      has(target, prop) {
        if (prop === "unregisterListener") return true;
        return prop in target;
      },
    }
  );

  (window as any).__TAURI_INTERNALS__ = new Proxy(
    {},
    {
      get(target, prop) {
        if (prop === "invoke") {
          return (target as any).invoke || (async () => {});
        }
        return (target as any)[prop];
      },
    }
  );
}


