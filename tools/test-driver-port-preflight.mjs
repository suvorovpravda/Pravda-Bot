// Real socket preflight without any HTTP/WebDriver dependency.
// Run on Linux and Windows runners with node tools/test-driver-port-preflight.mjs.
import assert from "node:assert/strict";
import { createServer } from "node:net";
import { driverPortsAvailable, loopbackPortAvailable } from "./driver-port-preflight.mjs";

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
console.log("Installed WebDriver TCP port preflight regression OK");
