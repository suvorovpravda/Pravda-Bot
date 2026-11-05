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

async function randomFreePort() {
  return new Promise((resolve, reject) => {
    const s = createServer();
    s.once("error", reject);
    s.listen({host:"127.0.0.1",port:0,exclusive:true},()=>{
      const address=s.address();
      const port=typeof address==="object" && address ? address.port : 0;
      s.close(error=>error ? reject(error) : resolve(port));
    });
  });
}

// Auto-select two distinct ephemeral ports on each driver restart. Respect
// explicit overrides, but refuse collisions rather than changing them.
export async function allocateDriverPorts(proxy=null, native=null) {
  for(const n of [proxy,native]) {
    if(n!==null && (!Number.isInteger(n)||n<1024||n>65535))
      throw Error("Invalid requested private WebDriver port");
  }
  for(let attempt=0;attempt<8;attempt++){
    const p=proxy ?? await randomFreePort();
    const n=native ?? await randomFreePort();
    if(p!==n && await driverPortsAvailable(p,n)) return {port:p,nativePort:n};
    if(proxy!==null && native!==null) break;
  }
  throw Error("Private WebDriver ports unavailable or not distinct");
}
