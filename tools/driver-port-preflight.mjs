// Dependency-free, fail-closed TCP preflight for the private WebDriver ports.
// An HTTP /status check alone cannot detect a non-WebDriver listener.
import { createServer } from "node:net";

export function loopbackPortAvailable(port) {
  if (!Number.isInteger(port) || port < 1024 || port > 65535)
    return Promise.resolve(false);
  return new Promise(resolve => {
    const server = createServer();
    let settled = false;
    const done = free => {
      if (settled) return;
      settled = true;
      resolve(free);
    };
    server.once("error", () => done(false));
    server.listen({host: "127.0.0.1", port, exclusive: true}, () => {
      server.close(error => done(!error));
    });
  });
}

export async function driverPortsAvailable(port, nativePort) {
  if (port === nativePort) return false;
  const [proxy, native] = await Promise.all([
    loopbackPortAvailable(port), loopbackPortAvailable(nativePort)
  ]);
  return proxy && native;
}
