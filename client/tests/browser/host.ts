import { spawn } from "node:child_process";
import { createServer } from "node:net";
import { join } from "node:path";

import { standIn } from "./stand-in";

import type { ChildProcess } from "node:child_process";
import type { AddressInfo } from "node:net";
import type { StandIn } from "./stand-in";

// The screens as a browser test reaches them: the bundle's dev host, its relay pointed at the stand-in of the
// voc service, which the test drives through `service`. `down` ends the dev host with every process it started, and
// the stand-in.

const DEV_APP = join(import.meta.dirname, "..", "..", "dev-app");
// How long the dev host is given to build and answer its first page.
const HOST_ANSWERS_MS = 600_000;
const ASK_EVERY_MS = 500;
// How much of what the dev host said is kept to quote when it fails.
const QUOTED_CHARS = 4_000;

export interface Host {
  address: string;
  service: StandIn;
  down: () => Promise<void>;
}

async function freePort(): Promise<number> {
  return new Promise((resolve, reject) => {
    const server = createServer();

    server.once("error", reject);
    server.listen(0, "127.0.0.1", () => {
      const { port } = server.address() as AddressInfo;

      server.close(() => resolve(port));
    });
  });
}

// Ends a process's whole group and waits for its head to be gone.
async function ended(child: ChildProcess, exited: () => boolean): Promise<void> {
  if (child.pid === undefined || exited()) return;

  const gone = new Promise<void>((resolve) => child.once("exit", () => resolve()));

  try {
    process.kill(-child.pid, "SIGKILL");
  } catch {
    // The group is already gone.
  }

  await gone;
}

/** Starts the stand-in and the dev host relaying to it, and waits until the voc screen answers. */
export async function stoodUp(): Promise<Host> {
  const service = standIn();
  const port = await freePort();
  const address = `http://127.0.0.1:${port}`;
  const child = spawn("bun", ["x", "nuxt", "dev", "--host", "127.0.0.1", "--port", String(port)], {
    cwd: DEV_APP,
    env: { ...process.env, BMO_DEV_VOC_ADDRESS: service.address, BMO_DEV_VOC_TOKEN: "test-token" },
    detached: true,
    stdio: ["ignore", "pipe", "pipe"]
  });
  let said = "";
  let exited = false;
  const keep = (chunk: Buffer): void => {
    said = `${said}${chunk.toString()}`.slice(-QUOTED_CHARS);
  };

  child.stdout?.on("data", keep);
  child.stderr?.on("data", keep);
  child.once("exit", () => {
    exited = true;
  });

  const down = async (): Promise<void> => {
    await ended(child, () => exited);
    service.stop();
  };

  try {
    const deadline = Date.now() + HOST_ANSWERS_MS;

    for (;;) {
      if (exited) throw new Error(`the dev host ended before it answered:\n${said}`);

      try {
        if ((await fetch(`${address}/voc`)).ok) break;
      } catch {
        // Not listening yet.
      }

      if (Date.now() > deadline) throw new Error(`the dev host did not answer within ${HOST_ANSWERS_MS} ms:\n${said}`);

      await Bun.sleep(ASK_EVERY_MS);
    }

    return { address, service, down };
  } catch (why) {
    await down();
    throw why;
  }
}
