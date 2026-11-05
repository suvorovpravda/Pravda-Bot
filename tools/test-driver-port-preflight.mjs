// Real socket preflight without any HTTP/WebDriver dependency.
// Run on Linux and Windows runners with node tools/test-driver-port-preflight.mjs.
import assert from "node:assert/strict";
import { createServer } from "node:net";
import { allocateDriverPorts, driverPortsAvailable, loopbackPortAvailable } from "./driver-port-preflight.mjs";

const occupied = createServer(socket => {
  // Non-WebDriver service must still be detected as an occupied TCP port.
  socket.end("HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n");
});
await new Promise((resolve, reject) => {
  occupied.once("error", reject);
  occupied.listen(0, "127.0.0.1", resolve);
});
const occupiedPort = occupied.address().port;
assert.equal(await loopbackPortAvailable(occupiedPort), false,
  "Unexpected TCP listener must be considered occupied");
assert.equal(await driverPortsAvailable(occupiedPort, 4445), false,
  "Never mistake an occupied non-WebDriver port for an available server");
assert.equal(await driverPortsAvailable(occupiedPort, occupiedPort), false,
  "Driver and native EdgeDriver cannot share the same port");
assert.equal(await driverPortsAvailable(-1, 4445), false,
  "Invalid ports must fail closed");
await new Promise((resolve, reject) => occupied.close(error => error ? reject(error) : resolve()));
assert.equal(await loopbackPortAvailable(occupiedPort), true,
  "Released loopback port must be available");
const chosen = await allocateDriverPorts();
assert.notEqual(chosen.port, chosen.nativePort, "OS-selected ports must differ");
assert.equal(await driverPortsAvailable(chosen.port, chosen.nativePort), true,
  "Two new OS-selected ports should be free");
await assert.rejects(allocateDriverPorts(-1, null), /Invalid requested/);
await assert.rejects(allocateDriverPorts(chosen.port, chosen.port), /unavailable|distinct/);
const oneFixed = await allocateDriverPorts(chosen.port, null);
assert.equal(oneFixed.port, chosen.port, "Explicit ports must not be silently replaced");
assert.notEqual(oneFixed.port, oneFixed.nativePort);
console.log("Installed WebDriver TCP port preflight regression OK");
